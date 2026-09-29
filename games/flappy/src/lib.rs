//! Playing Flappy without a window.
//!
//! Gameplay lives in the scene and Decay under `assets/`. This harness assembles
//! the same public engine pieces a host uses so tests can play the project.

use std::path::{Path, PathBuf};
use std::time::Duration;

use sindri_core::{
    ComponentSchemaRegistry, EntityId, SceneDocument, UnknownComponentPolicy, World,
};
use sindri_decay::{Physics2d, ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::{InputEvent, InputState, Key};
use sindri_scene::{SceneExtractor, ScenePhysics2d, SpriteAnimations};

#[must_use]
pub fn project() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

pub struct Run {
    pub world: World,
    pub components: ComponentSchemaRegistry,
    pub scripts: Scripts,
    pub sources: ScriptSources,
    pub physics: ScenePhysics2d,
    pub animations: SpriteAnimations,
    pub input: InputState,
}

impl Run {
    pub fn open() -> Result<Self, String> {
        let root = project().join("assets");
        let text = std::fs::read_to_string(root.join("flappy.scene.json"))
            .map_err(|error| error.to_string())?;
        let document: SceneDocument =
            serde_json::from_str(&text).map_err(|error| error.to_string())?;
        document.validate().map_err(|error| error.to_string())?;

        // Check component data the way the renderer will, so a typo such as an
        // unknown shape kind fails here instead of in the browser.
        let extractor = SceneExtractor::new().map_err(|error| error.to_string())?;
        extractor
            .validate(&document, UnknownComponentPolicy::Preserve)
            .map_err(|error| format!("{error}: {error:?}"))?;
        let mut components = extractor.components().clone();
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

        Ok(Self {
            world,
            components,
            scripts: Scripts::new(),
            sources,
            physics: ScenePhysics2d::top_down().map_err(|error| error.to_string())?,
            animations: SpriteAnimations::new(),
            input: InputState::default(),
        })
    }

    pub fn step(&mut self, delta: f32) -> Vec<String> {
        let step = Duration::from_secs_f32(delta);
        let mut notes = Vec::new();
        if let Err(error) = self.physics.step(&mut self.world, &self.components, step) {
            notes.push(error.to_string());
        }
        let (physics, events) = self.physics.for_scripts();
        let report = self.scripts.advance(
            &mut self.world,
            &self.components,
            ScriptFrame::new(&self.sources, &self.input, delta)
                .with_physics(Physics2d {
                    world: physics,
                    events,
                })
                .with_animations(&mut self.animations),
        );
        notes.extend(report.failures.iter().map(ToString::to_string));
        if let Err(error) = self
            .animations
            .advance(&self.world, &self.components, delta)
        {
            notes.push(error.to_string());
        }
        self.input.begin_frame(step);
        notes
    }

    pub fn key(&mut self, key: Key, down: bool) {
        self.input.apply(if down {
            InputEvent::KeyPressed(key)
        } else {
            InputEvent::KeyReleased(key)
        });
    }

    #[must_use]
    pub fn board(&self, name: &str) -> f32 {
        #[allow(clippy::cast_possible_truncation)]
        let value = self.scripts.blackboard().get(name, 0.0) as f32;
        value
    }

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

    #[must_use]
    pub fn position(&self, entity: EntityId) -> [f32; 2] {
        self.world
            .get(entity)
            .and_then(|data| data.transform_3d)
            .map_or([0.0, 0.0], sindri_core::Transform3D::position_2d)
    }
}
