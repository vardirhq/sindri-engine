//! Any project played offscreen the way the browser plays it: the same
//! session, the same Weave styling for the viewport, the same draw, with
//! clicks, keys and wheel delivered as a host delivers them.
//!
//! What `project-capture` photographs and `project-benchmark` times. It reads
//! a project from its directory rather than from what the native game embeds,
//! so any project in the repository can be opened, not only Causeway.

use std::{collections::BTreeMap, error::Error, fs, path::Path, time::Duration};

use sindri_assets::{AssetBytes, AssetDecoder, FontAssetDecoder};
use sindri_core::{
    AssetId, LoadedScenes, PREFAB_SUFFIX, PROFILE_SUFFIX, PrefabDocument, ProfileDocument,
    SceneDocument, SceneEntityId, TileSetDocument, World,
};
use sindri_decay::{PrefabSources, ProfileSources, ScriptSources};
use sindri_platform::{InputEvent, InputState, Key, MouseButton};
use sindri_render::TextRenderer;
use sindri_scene::{SceneExtractor, TileSetBindings, measure_ui_text};

use crate::{Session, extractor};

mod draw;

pub use draw::{DrawTimes, ProjectRenderer};

/// One fixed step at the rate every host runs gameplay.
pub const STEP: f32 = 1.0 / 60.0;

/// Every file under `root` with `extension`, by its asset ID.
fn files(root: &Path, extension: &str) -> BTreeMap<String, Vec<u8>> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.to_string_lossy().ends_with(extension)
                && let (Ok(relative), Ok(bytes)) = (path.strip_prefix(root), fs::read(&path))
            {
                found.insert(relative.to_string_lossy().replace('\\', "/"), bytes);
            }
        }
    }
    found
}

pub(crate) fn text(bytes: Vec<u8>) -> Result<String, Box<dyn Error>> {
    Ok(String::from_utf8(bytes)?)
}

/// What `sindri.toml` says: the main scene's ID, its entry stylesheets,
/// and the paths of its other scenes.
type Manifest = (String, Vec<String>, Vec<String>);

/// The project's main scene, its other scenes and its entry stylesheets,
/// from `sindri.toml`.
fn manifest(project: &Path) -> Result<Manifest, Box<dyn Error>> {
    let toml = fs::read_to_string(project.join("sindri.toml"))?;
    let quoted = |line: &str| -> Vec<String> {
        line.split('"')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect()
    };
    let scene = toml
        .lines()
        .find(|line| line.trim_start().starts_with("main_scene"))
        .and_then(|line| quoted(line).into_iter().next())
        .ok_or("sindri.toml names no main_scene")?;
    let scene = scene.strip_prefix("assets/").unwrap_or(&scene).to_owned();
    // The include list may run over several lines, up to its `]`.
    let include = toml
        .find("include")
        .map(|start| &toml[start..])
        .and_then(|rest| rest.split_once(']').map(|(list, _)| list))
        .unwrap_or_default();
    let sheets = quoted(include)
        .into_iter()
        .filter(|id| {
            Path::new(id)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("weave"))
        })
        .collect();
    // Other scenes `Scene.go` can reach, kept as their paths under the
    // project; the list may also run over several lines.
    let others = toml
        .lines()
        .position(|line| line.trim_start().starts_with("scenes"))
        .map(|start| {
            let rest = toml.lines().skip(start).collect::<Vec<_>>().join("\n");
            let list = rest.split_once(']').map_or(rest.as_str(), |(list, _)| list);
            quoted(list)
        })
        .unwrap_or_default();
    Ok((scene, sheets, others))
}

/// A pixel over the middle of `name`, as the last step laid it out.
fn pixel_of(session: &Session, world: &World, name: &str, size: [f32; 2]) -> Option<[f32; 2]> {
    let entity = world.entity_for_source_id(&SceneEntityId::new(name).ok()?)?;
    let rect = session.screen_ui().rect(entity)?;
    let half = size[1] / 2.0;
    Some([
        size[0] / 2.0 + rect.center[0] * half,
        half - rect.center[1] * half,
    ])
}

pub struct ProjectPlayer {
    pub session: Session,
    pub world: World,
    pub input: InputState,
    pub scene: SceneExtractor,
    pub text: TextRenderer,
    /// The screen it plays on, in pixels.
    pub size: [f32; 2],
    /// The project's block and tile sets, which the browser host binds
    /// from the export manifest and the scripts and world read.
    pub tile_sets: TileSetBindings,
}

impl ProjectPlayer {
    pub const fn viewport(&self) -> weave::Viewport {
        weave::Viewport {
            width: self.size[0],
            height: self.size[1],
        }
    }

    /// One fixed step and one draw's worth of styling, as a host runs.
    pub fn step(&mut self) -> Result<(), Box<dyn Error>> {
        self.advance()?;
        let view = self.viewport();
        let undo = self.session.style(&mut self.world, view)?;
        let sizes = measure_ui_text(&self.world, self.scene.components(), &mut self.text)?;
        self.session.record_drawn(&self.world, view, sizes)?;
        if let Some(undo) = undo {
            undo.undo(&mut self.world);
        }
        Ok(())
    }

    /// One fixed step of gameplay and nothing drawn: what a host's update
    /// does, with the draw left to whoever draws.
    pub fn advance(&mut self) -> Result<(), Box<dyn Error>> {
        let viewport = (self.size[0], self.size[1]);
        self.session
            .step(&mut self.world, &self.input, viewport, STEP)?;
        self.input.begin_frame(Duration::from_secs_f32(STEP));
        Ok(())
    }

    pub fn play(&mut self, seconds: f32) -> Result<(), Box<dyn Error>> {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let steps = (seconds / STEP).ceil().max(1.0) as usize;
        for _ in 0..steps {
            self.step()?;
        }
        Ok(())
    }

    fn point_at(&mut self, name: &str) -> Result<(), Box<dyn Error>> {
        let [x, y] = pixel_of(&self.session, &self.world, name, self.size)
            .ok_or_else(|| format!("{name} is not laid out on screen"))?;
        println!("{name} at ({x:.0}, {y:.0})");
        self.input.apply(InputEvent::PointerMoved { x, y });
        Ok(())
    }

    pub fn perform(&mut self, action: &str) -> Result<(), Box<dyn Error>> {
        let (verb, rest) = action.split_once(':').unwrap_or((action, ""));
        match verb {
            "click" => {
                self.point_at(rest)?;
                self.input
                    .apply(InputEvent::ButtonPressed(MouseButton::Left));
                self.step()?;
                self.input
                    .apply(InputEvent::ButtonReleased(MouseButton::Left));
                self.step()?;
            }
            "wheel" => {
                let (name, pixels) = rest.rsplit_once(':').ok_or("wheel:<entity>:<px>")?;
                self.point_at(name)?;
                let pixels: f32 = pixels.parse()?;
                self.input
                    .apply(InputEvent::Scrolled { x: 0.0, y: -pixels });
                self.step()?;
            }
            "key" => {
                let key = Key::from_name(rest).ok_or_else(|| format!("no key {rest}"))?;
                self.input.apply(InputEvent::KeyPressed(key));
                self.step()?;
                self.input.apply(InputEvent::KeyReleased(key));
                self.step()?;
            }
            "type" => {
                for c in rest.chars() {
                    self.input.apply(InputEvent::TextInput(c));
                }
                self.step()?;
            }
            "wait" => self.play(rest.parse()?)?,
            // A pointer at a pixel, and the left button pressed or let
            // go there: enough to drag something across the world.
            "move" => {
                let (x, y) = rest.split_once(',').ok_or("move:<x>,<y>")?;
                let (x, y) = (x.parse()?, y.parse()?);
                self.input.apply(InputEvent::PointerMoved { x, y });
                self.step()?;
            }
            "down" => {
                self.input
                    .apply(InputEvent::ButtonPressed(MouseButton::Left));
                self.step()?;
            }
            "up" => {
                self.input
                    .apply(InputEvent::ButtonReleased(MouseButton::Left));
                self.step()?;
            }
            "set" => {
                let (name, value) = rest.split_once('=').ok_or("set:<name>=<value>")?;
                self.session.set_board(name, value.parse()?);
                self.step()?;
            }
            _ => return Err(format!("unknown step {action}").into()),
        }
        Ok(())
    }
}

/// Everything opened: the player, and the images and sheets a renderer
/// binds, by asset ID.
pub struct OpenedProject {
    pub player: ProjectPlayer,
    pub images: BTreeMap<String, Vec<u8>>,
    pub sheets: BTreeMap<String, Vec<u8>>,
}

/// Opens the project at `project` to play on a screen of `size` pixels.
pub fn open(project: &Path, size: [f32; 2]) -> Result<OpenedProject, Box<dyn Error>> {
    let assets = project.join("assets");
    let (scene_id, sheet_ids, other_scenes) = manifest(project)?;
    let document = SceneDocument::from_json(&fs::read_to_string(assets.join(&scene_id))?)?;

    let mut sources = ScriptSources::new();
    for (id, bytes) in files(&assets, ".decay") {
        sources.insert(id, text(bytes)?);
    }
    let mut prefabs = PrefabSources::new();
    for (id, bytes) in files(&assets, PREFAB_SUFFIX) {
        prefabs.insert(id, PrefabDocument::from_json(&text(bytes)?)?);
    }
    let mut world = World::default();
    let mut loaded = LoadedScenes::new();
    loaded.enter_keeping_identities_with(&mut world, &scene_id, &document, &prefabs)?;
    let mut profiles = ProfileSources::new();
    for (id, bytes) in files(&assets, PROFILE_SUFFIX) {
        profiles.insert(id, ProfileDocument::from_json(&text(bytes)?)?);
    }
    let mut tile_sets = TileSetBindings::new();
    crate::bind_builtin_tile_sets(&mut tile_sets)?;
    for (id, bytes) in files(&assets, ".tileset") {
        tile_sets.bind(&id, TileSetDocument::from_json(&text(bytes)?)?)?;
    }
    let weave_sources: BTreeMap<String, String> = files(&assets, ".weave")
        .into_iter()
        .map(|(id, bytes)| Ok((id, text(bytes)?)))
        .collect::<Result<_, Box<dyn Error>>>()?;
    let mut sheets = Vec::new();
    for id in sheet_ids {
        sheets.push(weave::compose(&id, &weave_sources)?);
    }

    // Every scene goes to the session under its file name, which is the
    // name `Scene.go` asks for, as the export names them.
    let mut scenes = vec![(scene_id, document)];
    for path in other_scenes {
        let name = Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("a scene path without a file name")?
            .to_owned();
        let other = SceneDocument::from_json(&fs::read_to_string(project.join(&path))?)?;
        scenes.push((name, other));
    }
    let scene = extractor()?;
    let mut session = Session::with_sources(scene.components().clone(), sources)
        .with_prefabs(prefabs)
        .with_profiles(profiles)
        .with_tile_sets(tile_sets.clone())
        .with_scenes(scenes, loaded)
        .with_styles(sheets);
    let view = weave::Viewport {
        width: size[0],
        height: size[1],
    };
    session.settle_styles(&mut world, view)?;
    let mut text = TextRenderer::new();
    for (id, bytes) in files(&assets, ".ttf") {
        let asset = FontAssetDecoder.decode(AssetBytes::new(id.parse::<AssetId>()?, bytes))?;
        text.bind_font(&id, asset.family(), asset.bytes().to_vec());
    }
    let player = ProjectPlayer {
        session,
        world,
        input: InputState::default(),
        scene,
        text,
        size,
        tile_sets,
    };
    Ok(OpenedProject {
        player,
        images: files(&assets, ".png"),
        sheets: files(&assets, ".sheet"),
    })
}
