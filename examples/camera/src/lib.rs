//! Playing the Camera Lab without a window.
//!
//! There is no game code here. The lab is its scene, its Decay and its
//! stylesheet in `assets/`; `sindri-player examples/camera` plays it in a
//! window, and the browser plays its export. This opens the project the way
//! both do, on the one runtime session, so the tests below play what they
//! play.

use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use sindri_runtime::ProjectRun;

/// Where the project is, from wherever the harness is being run.
#[must_use]
pub fn project() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// One session in the lab, on a screen of a given size.
pub struct Run(Box<ProjectRun>);

impl Run {
    /// Opens the lab on a screen of `width` by `height` pixels.
    ///
    /// # Errors
    /// If the project will not read, will not parse, or will not load.
    pub fn open(width: f32, height: f32) -> Result<Self, String> {
        ProjectRun::open(&project(), [width, height]).map(|run| Self(Box::new(run)))
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

#[cfg(test)]
mod tests;
