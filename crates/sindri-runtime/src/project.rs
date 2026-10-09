//! A project read from its directory and played: what a test harness, a
//! capture or a benchmark opens to play a project the way a build does.
//!
//! Every file under `assets/` is read — scripts, prefabs, profiles, tile
//! sets, stylesheets — and every scene `sindri.toml` names. The main scene is
//! entered under its asset ID, the others are handed to the session by file
//! name, which is what `Scene.go` asks for, as the export names them. Fonts and
//! images are a drawing host's, and are not read here.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use sindri_core::{
    EntityId, LoadedScenes, PREFAB_SUFFIX, PROFILE_SUFFIX, PrefabDocument, ProfileDocument,
    SceneDocument, SceneEntityId, TagsComponent, TileSetDocument, UnknownComponentPolicy, World,
};
use sindri_decay::{PrefabSources, ProfileSources, ScriptComponent, ScriptSources};
use sindri_platform::{GamepadAxis, GamepadButton, InputEvent, InputState, Key, PadId};
use sindri_scene::{SceneExtractor, TileSetBindings, UiTextSizes};

use crate::{Session, StepReport};

/// One fixed step at the rate every host runs gameplay.
pub const STEP: f32 = 1.0 / 60.0;

/// The engine's component schemas and the one scripting adds, which is what
/// every host reads a scene with.
///
/// # Errors
/// Only a build whose own components do not register.
pub fn scene_extractor() -> Result<SceneExtractor, String> {
    let mut extractor = SceneExtractor::new().map_err(|error| error.to_string())?;
    extractor
        .register::<ScriptComponent>("Script")
        .map_err(|error| error.to_string())?;
    Ok(extractor)
}

/// Every file under `root` whose name ends with `suffix`, by its asset ID.
#[must_use]
pub fn files_under(root: &Path, suffix: &str) -> BTreeMap<String, Vec<u8>> {
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
            } else if path.to_string_lossy().ends_with(suffix)
                && let (Ok(relative), Ok(bytes)) = (path.strip_prefix(root), fs::read(&path))
            {
                found.insert(relative.to_string_lossy().replace('\\', "/"), bytes);
            }
        }
    }
    found
}

fn text(id: &str, bytes: Vec<u8>) -> Result<String, String> {
    String::from_utf8(bytes).map_err(|error| format!("{id}: {error}"))
}

/// What `sindri.toml` says: the main scene's ID under `assets/`, its entry
/// stylesheets, and the paths of its other scenes.
pub type Manifest = (String, Vec<String>, Vec<String>);

/// The parts of `sindri.toml` a run reads; the rest of the file is for the
/// export and the editor (`docs/project-format.md`).
#[derive(serde::Deserialize)]
struct ProjectFile {
    project: ProjectSection,
    #[serde(default)]
    assets: AssetsSection,
}

#[derive(serde::Deserialize)]
struct ProjectSection {
    main_scene: String,
    #[serde(default)]
    scenes: Vec<String>,
}

#[derive(Default, serde::Deserialize)]
struct AssetsSection {
    #[serde(default)]
    include: Vec<String>,
}

/// Reads the parts of `sindri.toml` a run needs, parsed as TOML as the export
/// parses it, so the two cannot read one file differently.
///
/// # Errors
/// A manifest that will not read or names no main scene.
pub fn manifest(project: &Path) -> Result<Manifest, String> {
    let path = project.join("sindri.toml");
    let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let file: ProjectFile =
        toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    let scene = file
        .project
        .main_scene
        .strip_prefix("assets/")
        .unwrap_or(&file.project.main_scene)
        .to_owned();
    let sheets = file
        .assets
        .include
        .into_iter()
        .filter(|id| {
            Path::new(id)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("weave"))
        })
        .collect();
    Ok((scene, sheets, file.project.scenes))
}

/// What a scene is called to `Scene.go` and the session: its file name, as
/// the export and the editor name it, wherever under the project it sits.
fn scene_name(path: &str) -> Result<String, String> {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("{path}: a scene path without a file name"))
}

/// A project being played, without a window: its world, its session, and
/// the input a host would deliver.
pub struct ProjectRun {
    pub session: Session,
    pub world: World,
    pub input: InputState,
    /// The screen it plays on, in pixels: what the screen UI is laid out
    /// against and the stylesheets' media queries read.
    pub size: [f32; 2],
    /// The schemas its scenes are read with.
    pub scene: SceneExtractor,
}

impl ProjectRun {
    /// Opens the project at `project` to play on a screen of `size` pixels,
    /// settled on its stylesheets as a build starts.
    ///
    /// # Errors
    /// A file that will not read or parse, a scene that will not load, or a
    /// stylesheet that will not compose — each named.
    pub fn open(project: &Path, size: [f32; 2]) -> Result<Self, String> {
        Self::open_on(project, None, size)
    }

    /// Opens the project on another of its scenes, named as `Scene.go` names
    /// it, as a tool or a test does to start somewhere other than the main
    /// scene without playing its way there. The main scene stays reachable.
    ///
    /// # Errors
    /// As [`Self::open`], and a scene the project does not list.
    pub fn open_scene(project: &Path, scene: &str, size: [f32; 2]) -> Result<Self, String> {
        Self::open_on(project, Some(scene), size)
    }

    fn open_on(project: &Path, entry: Option<&str>, size: [f32; 2]) -> Result<Self, String> {
        let assets = project.join("assets");
        let (main_path, sheet_ids, other_scenes) = manifest(project)?;
        let main_id = scene_name(&main_path)?;
        let read_scene = |path: &Path| -> Result<SceneDocument, String> {
            let json =
                fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
            SceneDocument::from_json(&json).map_err(|error| format!("{}: {error}", path.display()))
        };
        // Every scene goes to the session under its file name, which is the
        // name `Scene.go` asks for, as the export names them.
        let mut scenes = vec![(main_id.clone(), read_scene(&assets.join(&main_path))?)];
        for path in other_scenes {
            scenes.push((scene_name(&path)?, read_scene(&project.join(&path))?));
        }
        // Component data is checked the way the renderer will read it, so a
        // typo such as an unknown shape kind fails when the run opens rather
        // than first in a browser.
        let scene = scene_extractor()?;
        for (name, document) in &scenes {
            scene
                .validate(document, UnknownComponentPolicy::Preserve)
                .map_err(|error| format!("{name}: {error}"))?;
        }
        let scene_id = entry.unwrap_or(&main_id).to_owned();
        let document = scenes
            .iter()
            .find(|(name, _)| *name == scene_id)
            .map(|(_, document)| document.clone())
            .ok_or_else(|| format!("sindri.toml lists no scene {scene_id}"))?;

        let mut sources = ScriptSources::new();
        for (id, bytes) in files_under(&assets, ".decay") {
            let source = text(&id, bytes)?;
            sources.insert(id, source);
        }
        let mut prefabs = PrefabSources::new();
        for (id, bytes) in files_under(&assets, PREFAB_SUFFIX) {
            let prefab = PrefabDocument::from_json(&text(&id, bytes)?)
                .map_err(|error| format!("{id}: {error}"))?;
            prefabs.insert(id, prefab);
        }
        let mut world = World::default();
        let mut loaded = LoadedScenes::new();
        loaded
            .enter_keeping_identities_with(&mut world, &scene_id, &document, &prefabs)
            .map_err(|error| error.to_string())?;
        let mut profiles = ProfileSources::new();
        for (id, bytes) in files_under(&assets, PROFILE_SUFFIX) {
            let profile = ProfileDocument::from_json(&text(&id, bytes)?)
                .map_err(|error| format!("{id}: {error}"))?;
            profiles.insert(id, profile);
        }
        let mut tile_sets = TileSetBindings::new();
        for (id, bytes) in files_under(&assets, ".tileset") {
            let tile_set = TileSetDocument::from_json(&text(&id, bytes)?)
                .map_err(|error| format!("{id}: {error}"))?;
            tile_sets
                .bind(&id, tile_set)
                .map_err(|error| error.to_string())?;
        }
        let mut weave_sources = BTreeMap::new();
        for (id, bytes) in files_under(&assets, ".weave") {
            let source = text(&id, bytes)?;
            weave_sources.insert(id, source);
        }
        let mut sheets = Vec::new();
        for id in sheet_ids {
            sheets.push(
                weave::compose(&id, &weave_sources).map_err(|error| format!("{id}: {error}"))?,
            );
        }
        let mut session = Session::with_sources(scene.components().clone(), sources)
            .with_prefabs(prefabs)
            .with_profiles(profiles)
            .with_tile_sets(tile_sets)
            .with_scenes(scenes, loaded)
            .with_styles(sheets);
        session
            .settle_styles(
                &mut world,
                weave::Viewport {
                    width: size[0],
                    height: size[1],
                },
            )
            .map_err(|error| error.to_string())?;
        Ok(Self {
            session,
            world,
            input: InputState::default(),
            size,
            scene,
        })
    }

    /// One fixed step of gameplay, then the input moves on to the next
    /// frame and the frame is presented, as a host's update and draw do.
    /// Nothing is drawn, but a styled game is laid out as it would be.
    ///
    /// # Errors
    /// What stops a run: a solver, a scene change, a stylesheet.
    pub fn step(&mut self, delta: f32) -> Result<StepReport, String> {
        let report = self
            .session
            .step(
                &mut self.world,
                &self.input,
                (self.size[0], self.size[1]),
                delta,
            )
            .map_err(|error| error.to_string())?;
        self.input.begin_frame(Duration::from_secs_f32(delta));
        self.present()?;
        Ok(report)
    }

    /// What a host does around a draw, without drawing: a styled game is
    /// styled in place for the frame, its screen laid out as shown, which the
    /// next step's clicks are hit-tested against, and the styling undone.
    /// Without this a styled run kept the layout it opened with, and a click
    /// on a screen that had since changed met nothing.
    ///
    /// # Errors
    /// A stylesheet that will not apply.
    pub fn present(&mut self) -> Result<(), String> {
        if !self.session.is_styled() {
            return Ok(());
        }
        let view = weave::Viewport {
            width: self.size[0],
            height: self.size[1],
        };
        let undo = self
            .session
            .style(&mut self.world, view)
            .map_err(|error| error.to_string())?;
        let laid_out = self
            .session
            .record_drawn(&self.world, view, UiTextSizes::new())
            .map_err(|error| error.to_string());
        if let Some(undo) = undo {
            undo.undo(&mut self.world);
        }
        laid_out
    }

    /// Holds or lets go of a key, as the window would.
    pub fn key(&mut self, key: Key, down: bool) {
        self.input.apply(if down {
            InputEvent::KeyPressed(key)
        } else {
            InputEvent::KeyReleased(key)
        });
    }

    /// Plugs in a pad, as the platform would report it.
    pub fn connect(&mut self, pad: u32) {
        self.input.apply(InputEvent::GamepadConnected(PadId(pad)));
    }

    /// Unplugs a pad.
    pub fn disconnect(&mut self, pad: u32) {
        self.input
            .apply(InputEvent::GamepadDisconnected(PadId(pad)));
    }

    /// Presses or lets go of a pad's button.
    pub fn button(&mut self, pad: u32, button: GamepadButton, down: bool) {
        let pad = PadId(pad);
        self.input.apply(if down {
            InputEvent::GamepadPressed { pad, button }
        } else {
            InputEvent::GamepadReleased { pad, button }
        });
    }

    /// Moves a pad's stick or trigger to `value`.
    pub fn axis(&mut self, pad: u32, axis: GamepadAxis, value: f32) {
        self.input.apply(InputEvent::GamepadAxisMoved {
            pad: PadId(pad),
            axis,
            value,
        });
    }

    /// Every active entity carrying `tag`, in the world's order.
    #[must_use]
    pub fn tagged(&self, tag: &str) -> Vec<EntityId> {
        let components = self.session.components();
        self.world
            .entities()
            .filter(|(entity, _)| {
                self.world.is_active(*entity)
                    && components
                        .get::<TagsComponent>(&self.world, *entity)
                        .ok()
                        .flatten()
                        .is_some_and(|tags| tags.has(tag))
            })
            .map(|(entity, _)| entity)
            .collect()
    }

    /// Where the middle of a screen element was laid out at the last step,
    /// in pixels from the top left, for a pointer or finger to press it.
    #[must_use]
    pub fn on_screen(&self, entity: EntityId) -> Option<[f32; 2]> {
        let rect = self.session.screen_ui().rect(entity)?;
        let [width, height] = self.size;
        let half_height = height / 2.0;
        Some([
            width / 2.0 + rect.center[0] * half_height,
            half_height - rect.center[1] * half_height,
        ])
    }

    /// What a script left on the shared board.
    #[must_use]
    pub fn board(&self, name: &str) -> f32 {
        #[allow(clippy::cast_possible_truncation)]
        let value = self.session.scripts().blackboard().get(name, 0.0) as f32;
        value
    }

    /// The entity a scene gave this stable ID.
    #[must_use]
    pub fn entity(&self, id: &str) -> Option<EntityId> {
        self.world
            .entity_for_source_id(&SceneEntityId::new(id.to_owned()).ok()?)
    }

    /// Where an entity is, in the plane.
    #[must_use]
    pub fn position(&self, entity: EntityId) -> [f32; 2] {
        self.world
            .get(entity)
            .and_then(|data| data.transform_3d)
            .map_or([0.0, 0.0], sindri_core::Transform3D::position_2d)
    }
}
