//! Playing DRIFT without a window.
//!
//! There is no game code here. What the game does is in `assets/`, its scenes
//! and its Decay; this opens the project the way every host does, with the one
//! runtime session every host steps, so a test plays what a build plays.

use std::path::{Path, PathBuf};

use sindri_platform::{InputEvent, Key};
use sindri_runtime::{ProjectRun, STEP};

/// Where the project is, from wherever the harness is being run.
#[must_use]
pub fn project() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Where `open_in_hold` puts the diver: eye level with the aft handhold.
/// `art/hold.py` names the same place `HOLD_START`.
pub const HOLD_START: [f32; 3] = [-6.3, -0.6, 0.0];

/// How many fixed steps make up about `seconds` of game time.
fn steps(seconds: f32) -> u32 {
    let count = (seconds.max(0.0) / STEP).ceil();
    // A test waits seconds, never long enough for this to saturate.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = count as u32;
    count
}

/// One run of the game, as a host plays it, with everything it has printed.
pub struct Run {
    /// The session, its world and its input.
    pub run: Box<ProjectRun>,
    /// Every line the scripts have printed so far.
    pub said: Vec<String>,
}

impl Run {
    /// Opens the project: every scene, script and prefab in it.
    ///
    /// # Errors
    /// If the project will not read, will not parse, or will not load.
    pub fn open() -> Result<Self, String> {
        let run = ProjectRun::open(&project(), [1280.0, 720.0])?;
        Ok(Self {
            run: Box::new(run),
            said: Vec::new(),
        })
    }

    /// Opens the project with the diver by the aft handhold in the hold,
    /// rather than waking in the cabin: where the floating and tether tests
    /// start. The diver holds on and clips on there, as at any loose end.
    ///
    /// # Errors
    /// As [`Run::open`].
    ///
    /// # Panics
    /// If the scene has no diver.
    pub fn open_in_hold() -> Result<Self, String> {
        let mut game = Self::open()?;
        let diver = game.run.entity("diver").expect("the scene has a diver");
        game.run
            .world
            .get_mut(diver)
            .and_then(|data| data.transform_3d.as_mut())
            .expect("the diver has a transform")
            .position = HOLD_START;
        Ok(game)
    }

    /// One fixed step.
    ///
    /// # Panics
    /// If the step fails or reports a problem: a playthrough has none.
    pub fn step(&mut self) {
        let report = self.run.step(STEP).expect("the game steps");
        assert!(report.notes().is_empty(), "{:?}", report.notes());
        self.said
            .extend(report.scripts.printed.into_iter().map(|line| line.message));
    }

    /// Steps for about `seconds` of game time.
    pub fn wait(&mut self, seconds: f32) {
        for _ in 0..steps(seconds) {
            self.step();
        }
    }

    /// Steps until a script prints `line`, for at most `seconds`; whether it did.
    pub fn until(&mut self, line: &str, seconds: f32) -> bool {
        for _ in 0..steps(seconds) {
            if self.has_said(line) {
                return true;
            }
            self.step();
        }
        self.has_said(line)
    }

    /// Whether a script has printed `line`.
    #[must_use]
    pub fn has_said(&self, line: &str) -> bool {
        self.said.iter().any(|said| said == line)
    }

    /// Presses a key down and leaves it held.
    pub fn press(&mut self, key: Key) {
        self.run.input.apply(InputEvent::KeyPressed(key));
    }

    /// Lets a held key go.
    pub fn release(&mut self, key: Key) {
        self.run.input.apply(InputEvent::KeyReleased(key));
    }

    /// Taps a key: down for one step, then up.
    pub fn tap(&mut self, key: Key) {
        self.press(key);
        self.step();
        self.release(key);
    }

    /// Where a scene entity is in the world, by its id.
    ///
    /// # Panics
    /// If there is no such entity.
    #[must_use]
    pub fn position(&self, id: &str) -> [f32; 3] {
        let entity = self.run.entity(id).expect("the entity exists");
        self.run
            .world
            .world_transform(entity)
            .expect("the entity has a transform")
            .position
    }
}
