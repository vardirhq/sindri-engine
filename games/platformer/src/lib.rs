//! Playing Lamplighter without a window.
//!
//! There is no game code here. What the game does is in `assets/`, the scene
//! and its Decay; this opens the project the way every host does, with the
//! one runtime session every host steps, so a test plays what a build plays.

use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use sindri_core::ComponentSchemaRegistry;
use sindri_decay::PrefabSources;
use sindri_runtime::ProjectRun;
use sindri_scene::ScenePhysics2d;

/// Where the project is, from wherever the harness is being run.
#[must_use]
pub fn project() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// The screen a run is played on, in pixels.
const SCREEN: [f32; 2] = [960.0, 540.0];

/// The scene the level tests play, and the level select opens first.
pub const FIRST_LEVEL: &str = "level-1.scene";

/// One run of the game: the project, played as a host plays it.
pub struct Run(Box<ProjectRun>);

impl Run {
    /// Opens the project on its first level, the way a person arrives there
    /// from the level select: every scene, script, prefab and profile in it.
    ///
    /// # Errors
    /// If the project will not read, will not parse, or will not load.
    pub fn open() -> Result<Self, String> {
        Self::open_scene(FIRST_LEVEL)
    }

    /// Opens the project as the shipped game starts, on its title screen.
    ///
    /// # Errors
    /// As [`Self::open`].
    pub fn open_title() -> Result<Self, String> {
        Self::open_title_in_viewport(SCREEN)
    }

    /// Opens the title with the dimensions of the screen being tested.
    ///
    /// # Errors
    /// As [`Self::open`].
    pub fn open_title_in_viewport(viewport: [f32; 2]) -> Result<Self, String> {
        ProjectRun::open(&project(), viewport).map(|run| Self(Box::new(run)))
    }

    /// Opens the project on one of its scenes, named as `Scene.go` names it.
    ///
    /// # Errors
    /// As [`Self::open`], and a scene the project does not list.
    pub fn open_scene(scene: &str) -> Result<Self, String> {
        ProjectRun::open_scene(&project(), scene, SCREEN).map(|run| Self(Box::new(run)))
    }

    /// One fixed step, returning every failure it reported.
    pub fn step(&mut self, delta: f32) -> Vec<String> {
        self.0
            .step(delta)
            .map_or_else(|error| vec![error], |report| report.notes())
    }

    /// The 2D solver the scene's Physics 2D World configures.
    #[must_use]
    pub const fn physics(&self) -> &ScenePhysics2d {
        self.0.session.physics()
    }

    #[must_use]
    pub const fn components(&self) -> &ComponentSchemaRegistry {
        self.0.session.components()
    }

    #[must_use]
    pub const fn prefabs(&self) -> &PrefabSources {
        self.0.session.prefabs()
    }
}

impl Deref for Run {
    type Target = ProjectRun;

    fn deref(&self) -> &ProjectRun {
        &self.0
    }
}

impl DerefMut for Run {
    fn deref_mut(&mut self) -> &mut ProjectRun {
        &mut self.0
    }
}
