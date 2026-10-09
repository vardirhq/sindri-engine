//! Playing Scorchball without a window.
//!
//! There is no game code here. What the game does is in `assets/`, the scene
//! and its Decay; this opens the project the way every host does, with the
//! one runtime session every host steps, so a test plays what a build plays.
//! Pads, tags and the board are the runtime's own test conveniences.

use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use sindri_core::EntityId;
use sindri_runtime::ProjectRun;

/// Where the project is, from wherever the harness is being run.
#[must_use]
pub fn project() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// One match: the project, played as a host plays it.
pub struct Run(Box<ProjectRun>);

impl Run {
    /// Opens the project: every scene, script and prefab in it.
    ///
    /// # Errors
    /// If the project will not read, will not parse, or will not load.
    pub fn open() -> Result<Self, String> {
        ProjectRun::open(&project(), [1280.0, 720.0]).map(|run| Self(Box::new(run)))
    }

    /// One fixed step, returning every failure it reported.
    pub fn step(&mut self, delta: f32) -> Vec<String> {
        self.0
            .step(delta)
            .map_or_else(|error| vec![error], |report| report.notes())
    }

    /// How big an entity is drawn: its scale on one axis.
    #[must_use]
    pub fn scale(&self, entity: EntityId) -> f32 {
        self.0
            .world
            .get(entity)
            .and_then(|data| data.transform_3d)
            .map_or(0.0, |transform| transform.scale[0])
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
