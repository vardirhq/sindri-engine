//! Playing Flappy without a window.
//!
//! There is no game code here. What the game does is in `assets/`, the scene
//! and its Decay; this opens the project the way every host does, with the
//! one runtime session every host steps, so a test plays what a build plays.

use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use sindri_runtime::ProjectRun;

/// Where the project is, from wherever the harness is being run.
#[must_use]
pub fn project() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// One run of the game: the project, played as a host plays it.
pub struct Run(Box<ProjectRun>);

impl Run {
    /// Opens the project: every scene and script in it.
    ///
    /// # Errors
    /// If the project will not read, will not parse, or will not load.
    pub fn open() -> Result<Self, String> {
        ProjectRun::open(&project(), [960.0, 540.0]).map(|run| Self(Box::new(run)))
    }

    /// One fixed step, returning every failure it reported.
    pub fn step(&mut self, delta: f32) -> Vec<String> {
        self.0
            .step(delta)
            .map_or_else(|error| vec![error], |report| report.notes())
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
