//! Plays the Physics Playground the way a browser plays it: the project read
//! off disk, styled by its own stylesheet, stepped by the shared session, with
//! clicks and keys delivered as a host delivers them.

use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Once,
    time::Duration,
};

use sindri_causeway::{Session, extractor};
use sindri_core::{
    EntityId, LoadedScenes, PrefabDocument, ProfileDocument, SceneDocument, SceneEntityId,
    TileSetDocument, World,
};
use sindri_decay::{PrefabSources, ProfileSources, ScriptSources, Scripts};
use sindri_platform::{InputEvent, InputState, Key, MouseButton};
use sindri_scene::{TileSetBindings, UiTextSizes};

pub const STEP: f32 = 1.0 / 60.0;

thread_local! {
    static LINES: RefCell<Vec<(log::Level, String)>> = const { RefCell::new(Vec::new()) };
}

/// Keeps every log line on the thread that wrote it, so tests running side by
/// side each read their own playground's prints and failures.
struct Capture;

impl log::Log for Capture {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Info
    }
    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            // PLAYGROUND_ECHO=1 shows every print while a test runs.
            if std::env::var_os("PLAYGROUND_ECHO").is_some() {
                eprintln!("{}", record.args());
            }
            LINES.with(|lines| {
                lines
                    .borrow_mut()
                    .push((record.level(), record.args().to_string()));
            });
        }
    }
    fn flush(&self) {}
}

static CAPTURE: Capture = Capture;
static INSTALL: Once = Once::new();

fn project() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/physics/assets")
}

/// Every file under `dir` with `extension`, by its asset ID.
fn files(extension: &str, dir: &str) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(project().join(dir)) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == extension) {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let text = std::fs::read_to_string(&path).expect("the project file reads");
            found.insert(format!("{dir}/{name}"), text);
        }
    }
    found
}

/// Whether any print on this thread's playground so far contained `words`.
pub fn said(words: &str) -> bool {
    LINES.with(|lines| lines.borrow().iter().any(|(_, line)| line.contains(words)))
}

/// How many prints on this thread's playground so far contained `words`.
pub fn times(words: &str) -> usize {
    LINES.with(|lines| {
        lines
            .borrow()
            .iter()
            .filter(|(_, line)| line.contains(words))
            .count()
    })
}

pub struct Playground {
    pub session: Session,
    pub world: World,
    pub input: InputState,
    pub size: [f32; 2],
}

impl Playground {
    pub fn open(width: f32, height: f32) -> Self {
        INSTALL.call_once(|| {
            log::set_logger(&CAPTURE).expect("one logger");
            log::set_max_level(log::LevelFilter::Info);
        });
        LINES.with(|lines| lines.borrow_mut().clear());
        let text =
            std::fs::read_to_string(project().join("playground.scene")).expect("the scene reads");
        let document = SceneDocument::from_json(&text).expect("the scene parses");
        let mut prefabs = PrefabSources::new();
        let mut placed = BTreeMap::new();
        for (id, text) in files("prefab", "prefabs") {
            let prefab = PrefabDocument::from_json(&text).expect("the prefab parses");
            placed.insert(id.clone(), prefab.clone());
            prefabs.insert(id, prefab);
        }
        let mut profiles = ProfileSources::new();
        for (id, text) in files("profile", "materials") {
            profiles.insert(
                id,
                ProfileDocument::from_json(&text).expect("the profile parses"),
            );
        }
        let mut world = World::default();
        let mut loaded = LoadedScenes::new();
        loaded
            .enter_keeping_identities_with(&mut world, "playground.scene", &document, &placed)
            .expect("the scene loads");
        let mut sources = ScriptSources::new();
        for (id, text) in files("decay", "scripts") {
            sources.insert(id, text);
        }
        let scene = extractor().expect("the schemas register");
        let failures = Scripts::new().compile(&world, scene.components(), &sources);
        assert!(failures.is_empty(), "{failures:?}");
        let weave: BTreeMap<String, String> = files("weave", "ui");
        let sheet = weave::compose("ui/playground.weave", &weave).expect("the stylesheet composes");
        // The 3D annex: its own scene, and the block set its voxels use.
        let quarry = SceneDocument::from_json(
            &std::fs::read_to_string(project().join("quarry.scene")).expect("the quarry reads"),
        )
        .expect("the quarry parses");
        let mut tile_sets = TileSetBindings::new();
        tile_sets
            .bind(
                "quarry.tileset",
                TileSetDocument::from_json(
                    &std::fs::read_to_string(project().join("quarry.tileset"))
                        .expect("the block set reads"),
                )
                .expect("the block set parses"),
            )
            .expect("the block set binds");
        let session = Session::with_sources(scene.components().clone(), sources)
            .with_prefabs(prefabs)
            .with_profiles(profiles)
            .with_tile_sets(tile_sets)
            .with_scenes(
                vec![
                    ("playground.scene".to_owned(), document),
                    ("quarry.scene".to_owned(), quarry),
                ],
                loaded,
            )
            .with_styles(vec![sheet]);
        let mut playground = Self {
            session,
            world,
            input: InputState::default(),
            size: [width, height],
        };
        let view = playground.viewport();
        playground
            .session
            .settle_styles(&mut playground.world, view)
            .expect("the stylesheet applies");
        playground.play(30);
        playground
    }

    fn viewport(&self) -> weave::Viewport {
        weave::Viewport {
            width: self.size[0],
            height: self.size[1],
        }
    }

    /// One fixed step and one draw, as the browser host runs them. A script
    /// failure is a test failure.
    pub fn step(&mut self) {
        self.session
            .step(
                &mut self.world,
                &self.input,
                (self.size[0], self.size[1]),
                STEP,
            )
            .expect("the step runs");
        self.input.begin_frame(Duration::from_secs_f32(STEP));
        let view = self.viewport();
        let undo = self
            .session
            .style(&mut self.world, view)
            .expect("the draw styles");
        self.session
            .record_drawn(&self.world, view, UiTextSizes::new())
            .expect("the draw records");
        if let Some(undo) = undo {
            undo.undo(&mut self.world);
        }
        let errors: Vec<String> = LINES.with(|lines| {
            lines
                .borrow()
                .iter()
                .filter(|(level, _)| *level == log::Level::Error)
                .map(|(_, line)| line.clone())
                .collect()
        });
        assert!(errors.is_empty(), "{errors:#?}");
    }

    pub fn play(&mut self, steps: usize) {
        for _ in 0..steps {
            self.step();
        }
    }

    pub fn id(&self, name: &str) -> EntityId {
        self.world
            .entity_for_source_id(&SceneEntityId::new(name).expect("a valid id"))
            .or_else(|| {
                self.world
                    .entities()
                    .find(|(_, data)| data.name.as_deref() == Some(name))
                    .map(|(entity, _)| entity)
            })
            .unwrap_or_else(|| panic!("no {name}"))
    }

    pub fn at(&self, name: &str) -> [f32; 2] {
        let position = self
            .world
            .world_transform(self.id(name))
            .expect("a transform")
            .position;
        [position[0], position[1]]
    }

    pub fn text(&self, name: &str) -> String {
        self.world.get(self.id(name)).expect("there").components["sindri.ui.text"]["text"]
            .as_str()
            .expect("words")
            .to_owned()
    }

    pub fn tagged(&self, tag: &str) -> usize {
        self.world
            .entities()
            .filter(|(entity, data)| {
                self.world.is_active(*entity)
                    && data.components.get("sindri.tags").is_some_and(|tags| {
                        tags["tags"]
                            .as_array()
                            .is_some_and(|list| list.iter().any(|t| t == tag))
                    })
            })
            .count()
    }

    fn overlay_to_pixel(&self, overlay: [f32; 2]) -> [f32; 2] {
        let half = self.size[1] / 2.0;
        [
            self.size[0] / 2.0 + overlay[0] * half,
            half - overlay[1] * half,
        ]
    }

    /// Where a world point is on the screen, from the camera the director
    /// last placed.
    pub fn pixel_of_world(&self, point: [f32; 2]) -> [f32; 2] {
        let camera = self.id("camera");
        let centre = self.at("camera");
        #[allow(clippy::cast_possible_truncation)]
        let height =
            self.world.get(camera).expect("camera").components["sindri.camera"]["vertical_size"]
                .as_f64()
                .expect("a size") as f32;
        let half = height / 2.0;
        self.overlay_to_pixel([(point[0] - centre[0]) / half, (point[1] - centre[1]) / half])
    }

    pub fn click(&mut self, name: &str) {
        let rect = self
            .session
            .screen_ui()
            .rect(self.id(name))
            .unwrap_or_else(|| panic!("{name} is not on screen"));
        let pixel = self.overlay_to_pixel(rect.center);
        self.press_at(pixel);
        self.release();
    }

    pub fn press_at(&mut self, pixel: [f32; 2]) {
        self.input.apply(InputEvent::PointerMoved {
            x: pixel[0],
            y: pixel[1],
        });
        self.step();
        self.input
            .apply(InputEvent::ButtonPressed(MouseButton::Left));
        self.step();
    }

    pub fn move_to(&mut self, pixel: [f32; 2]) {
        self.input.apply(InputEvent::PointerMoved {
            x: pixel[0],
            y: pixel[1],
        });
        self.step();
    }

    pub fn release(&mut self) {
        self.input
            .apply(InputEvent::ButtonReleased(MouseButton::Left));
        self.play(2);
    }

    pub fn key(&mut self, key: Key) {
        self.input.apply(InputEvent::KeyPressed(key));
        self.step();
        self.input.apply(InputEvent::KeyReleased(key));
        self.play(2);
    }
}
