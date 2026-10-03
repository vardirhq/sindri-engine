//! Playing Low Tide without a window.
//!
//! There is no game code here. What the game does is in `assets/`, the scene
//! and its Decay, and this module assembles the public pieces a host
//! assembles, in the order a host runs them, so a test can play it.

use std::path::{Path, PathBuf};
use std::time::Duration;

use sindri_core::{
    ComponentSchemaRegistry, EntityId, SceneDocument, TileSetDocument, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::{InputEvent, InputState, Key};
use sindri_scene::{SceneExtractor, ScreenExtent, ScreenUi, SpriteAnimations, TileSetBindings};

/// Where the project is, from wherever the harness is being run.
#[must_use]
pub fn project() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// What a finger does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finger {
    Down,
    Move,
    Up,
}

/// One voyage, held together as a host holds it.
pub struct Run {
    pub world: World,
    pub components: ComponentSchemaRegistry,
    pub scripts: Scripts,
    pub sources: ScriptSources,
    pub animations: SpriteAnimations,
    pub input: InputState,
    /// The phone's buttons are screen UI, so the harness lays it out and
    /// hit-tests it as a host does.
    pub screen_ui: ScreenUi,
    /// The screen being played on, in pixels.
    pub screen: [f32; 2],
    /// The Basin's block set, which scripts read the world through.
    pub tile_sets: TileSetBindings,
}

impl Run {
    /// Opens the project: its scene and every script in it.
    ///
    /// # Errors
    /// If the project will not read, will not parse, or will not load.
    pub fn open() -> Result<Self, String> {
        let root = project().join("assets");
        let text = std::fs::read_to_string(root.join("low-tide.scene"))
            .map_err(|error| error.to_string())?;
        let document: SceneDocument =
            serde_json::from_str(&text).map_err(|error| error.to_string())?;
        document.validate().map_err(|error| error.to_string())?;

        let mut components = SceneExtractor::new()
            .map_err(|error| error.to_string())?
            .components()
            .clone();
        components
            .register::<ScriptComponent>("Script")
            .map_err(|error| error.to_string())?;
        let world = World::from_scene(&document)
            .map_err(|error| error.to_string())?
            .world;

        let mut sources = ScriptSources::new();
        for entry in std::fs::read_dir(root.join("scripts")).map_err(|error| error.to_string())? {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "decay")
            {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                let text = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
                sources.insert(format!("scripts/{name}"), text);
            }
        }

        let mut tile_sets = TileSetBindings::new();
        let basin = std::fs::read_to_string(root.join("basin.tileset"))
            .map_err(|error| error.to_string())?;
        tile_sets
            .bind(
                "basin.tileset",
                TileSetDocument::from_json(&basin).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;

        Ok(Self {
            world,
            components,
            scripts: Scripts::new(),
            sources,
            animations: SpriteAnimations::new(),
            input: InputState::default(),
            screen_ui: ScreenUi::default(),
            screen: [1280.0, 720.0],
            tile_sets,
        })
    }

    /// One fixed step, returning every failure it reported.
    pub fn step(&mut self, delta: f32) -> Vec<String> {
        let mut notes = Vec::new();
        if let Err(error) = self.screen_ui.update(
            &mut self.world,
            &self.components,
            ScreenExtent::new(self.screen[0], self.screen[1]),
            self.input.presses(),
        ) {
            notes.push(error.to_string());
        }
        let report = self.scripts.advance(
            &mut self.world,
            &self.components,
            ScriptFrame::new(&self.sources, &self.input, delta)
                .with_screen_ui(&self.screen_ui)
                .with_tile_sets(&self.tile_sets)
                .with_animations(&mut self.animations),
        );
        notes.extend(report.failures.iter().map(ToString::to_string));
        if let Err(error) = self
            .animations
            .advance(&self.world, &self.components, delta)
        {
            notes.push(error.to_string());
        }
        self.input.begin_frame(Duration::from_secs_f32(delta));
        notes
    }

    /// Holds or lets go of a key, as the window would.
    pub fn key(&mut self, key: Key, down: bool) {
        self.input.apply(if down {
            InputEvent::KeyPressed(key)
        } else {
            InputEvent::KeyReleased(key)
        });
    }

    /// Puts a finger down, moves it or lifts it, at a point in pixels from
    /// the top left of the screen, as a touch screen would report it.
    pub fn finger(&mut self, id: u64, phase: Finger, at: [f32; 2]) {
        let [x, y] = at;
        self.input.apply(match phase {
            Finger::Down => InputEvent::TouchStarted { id, x, y },
            Finger::Move => InputEvent::TouchMoved { id, x, y },
            Finger::Up => InputEvent::TouchEnded { id },
        });
    }

    /// Where the middle of a screen element was laid out, in pixels from the
    /// top left, for a finger to press it.
    #[must_use]
    pub fn on_screen(&self, entity: EntityId) -> Option<[f32; 2]> {
        let rect = self.screen_ui.rect(entity)?;
        let half_height = self.screen[1] / 2.0;
        Some([
            self.screen[0] / 2.0 + rect.center[0] * half_height,
            half_height - rect.center[1] * half_height,
        ])
    }

    /// What a script left on the shared board: a number, or 1 for true.
    #[must_use]
    pub fn board(&self, name: &str) -> f32 {
        #[allow(clippy::cast_possible_truncation)]
        let value = self.scripts.blackboard().get(name, 0.0) as f32;
        value
    }

    /// The entity a scene gave this stable ID.
    #[must_use]
    pub fn entity(&self, id: &str) -> Option<EntityId> {
        self.world
            .entities()
            .find(|(_, data)| {
                data.source_id
                    .as_ref()
                    .is_some_and(|source| source.as_str() == id)
            })
            .map(|(entity, _)| entity)
    }

    /// Where an entity is relative to its parent, in the plane.
    #[must_use]
    pub fn local(&self, entity: EntityId) -> [f32; 2] {
        self.world
            .get(entity)
            .and_then(|data| data.transform_3d)
            .map_or([0.0, 0.0], Transform3D::position_2d)
    }

    /// Where an entity is in the world, every parent folded in.
    #[must_use]
    pub fn position(&self, entity: EntityId) -> [f32; 2] {
        self.world
            .world_transform(entity)
            .map_or([0.0, 0.0], Transform3D::position_2d)
    }
}
