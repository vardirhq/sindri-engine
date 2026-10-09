//! Playing Low Tide without a window.
//!
//! There is no game code here. What the game does is in `assets/`, the scene
//! and its Decay, and this module assembles the public pieces a host
//! assembles, in the order a host runs them, so a test can play it.

use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use sindri_core::{EntityId, Transform3D};
use sindri_platform::InputEvent;
use sindri_runtime::ProjectRun;

/// Where the project is, from wherever the harness is being run.
#[must_use]
pub fn project() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// What a finger does, as a touch screen reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finger {
    Down,
    Move,
    Up,
}

/// One voyage: the project, played as a host plays it, on a landscape screen
/// unless a test turns it.
pub struct Run(Box<ProjectRun>);

impl Run {
    /// Opens the project: every scene, script, prefab and tile set in it.
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

    /// Puts a finger down, moves it or lifts it, at a point in pixels from
    /// the top left of the screen, as a touch screen would report it.
    pub fn finger(&mut self, id: u64, phase: Finger, at: [f32; 2]) {
        let [x, y] = at;
        self.0.input.apply(match phase {
            Finger::Down => InputEvent::TouchStarted { id, x, y },
            Finger::Move => InputEvent::TouchMoved { id, x, y },
            Finger::Up => InputEvent::TouchEnded { id },
        });
    }

    /// Where an entity is relative to its parent, in the plane.
    #[must_use]
    pub fn local(&self, entity: EntityId) -> [f32; 2] {
        self.0
            .world
            .get(entity)
            .and_then(|data| data.transform_3d)
            .map_or([0.0, 0.0], Transform3D::position_2d)
    }

    /// Where an entity is in the world, every parent folded in.
    #[must_use]
    pub fn position(&self, entity: EntityId) -> [f32; 2] {
        self.0
            .world
            .world_transform(entity)
            .map_or([0.0, 0.0], Transform3D::position_2d)
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
