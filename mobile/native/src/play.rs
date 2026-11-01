//! Play from the editor: the project run for real, by the engine's own
//! `Session`, in place of the canvas, until Stop.
//!
//! The run is opened from the project's files, as a build opens it. That is
//! the scene being edited, because the app saves every edit as it is made; a
//! save that failed is the one way the two differ, and the app saves before
//! it plays. The run has a world of its own, so nothing it does reaches the
//! editor's world or its file: Stop drops it, and the scene is as it was.
//!
//! A phone has no keys, and Lamplighter (like most games here) reads its
//! movement from them. The design's answer is "drag anywhere to walk": a
//! stick under the thumb, which this turns into the keys a keyboard player
//! would hold, and a tap, which is the interact key.

use std::path::Path;

use sindri_platform::Key;
use sindri_runtime::{ProjectRun, STEP};
use sindri_scene::CameraView;

use crate::renderer::{SceneRenderer, Shot};

/// More steps than this in one frame are dropped rather than caught up: a
/// phone that stalled (a notification, a slow frame) carries on from where it
/// is instead of running the game fast to make up the time.
const MOST_STEPS: f32 = 5.0;

/// How far the stick must be pushed along an axis for its key to be held.
/// Below a full push on both axes, so a diagonal holds both keys.
const PUSHED: f32 = 0.35;

/// The keys the stick holds, by direction: right, left, up, down.
const KEYS: [Key; 4] = [Key::D, Key::A, Key::W, Key::S];

/// A game running from the editor.
pub struct Playing {
    run: ProjectRun,
    /// Time not yet stepped, below one fixed step.
    unspent: f32,
    seconds: f32,
    paused: bool,
    /// Which of [`KEYS`] the stick is holding.
    held: [bool; 4],
    /// The interact key went down this frame and comes up after one step.
    tapped: bool,
}

impl Playing {
    /// The project at `project` playing `scene` (its file name, such as
    /// `world.scene`) on a screen `size` pixels across.
    ///
    /// # Errors
    /// As [`ProjectRun::open_scene`]: a file that will not read, a scene that
    /// will not load, a stylesheet that will not compose — each named.
    pub fn start(project: &Path, scene: &str, size: [f32; 2]) -> Result<Self, String> {
        Ok(Self {
            run: ProjectRun::open_scene(project, scene, size)?,
            unspent: 0.0,
            seconds: 0.0,
            paused: false,
            held: [false; 4],
            tapped: false,
        })
    }

    /// Runs the game on by `elapsed` seconds of real time, in fixed steps.
    /// Nothing moves while paused.
    ///
    /// # Errors
    /// What stops a run: a solver, a scene change, a stylesheet.
    pub fn advance(&mut self, elapsed: f32) -> Result<(), String> {
        if self.paused || !elapsed.is_finite() || elapsed <= 0.0 {
            return Ok(());
        }
        self.unspent = (self.unspent + elapsed).min(STEP * MOST_STEPS);
        while self.unspent >= STEP {
            self.unspent -= STEP;
            self.step_once()?;
        }
        Ok(())
    }

    /// One fixed step, paused or not: the paused panel's step button.
    ///
    /// # Errors
    /// As [`Self::advance`].
    pub fn step_once(&mut self) -> Result<(), String> {
        self.run.step(STEP)?;
        self.seconds += STEP;
        if self.tapped {
            self.tapped = false;
            self.run.key(Key::E, false);
        }
        Ok(())
    }

    /// The stick pushed `direction` (right and up, each from -1 to 1), or let
    /// go with `None`: holds and lets go of the movement keys to match.
    pub fn stick(&mut self, direction: Option<[f32; 2]>) {
        let [x, y] = direction.unwrap_or([0.0, 0.0]);
        let wanted = [x > PUSHED, x < -PUSHED, y > PUSHED, y < -PUSHED];
        for ((key, held), want) in KEYS.into_iter().zip(&mut self.held).zip(wanted) {
            if *held != want {
                self.run.key(key, want);
                *held = want;
            }
        }
    }

    /// A tap on the game: the interact key, pressed for one step. Nothing
    /// while paused, so a tap then is not saved up for the resume.
    pub fn tap(&mut self) {
        if !self.tapped && !self.paused {
            self.run.key(Key::E, true);
            self.tapped = true;
        }
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        if paused {
            self.stick(None);
        }
    }

    pub const fn paused(&self) -> bool {
        self.paused
    }

    /// How long the game has run, in seconds of game time.
    pub const fn seconds(&self) -> f32 {
        self.seconds
    }

    /// The run, for a test to look into.
    pub const fn run(&self) -> &ProjectRun {
        &self.run
    }

    /// Draws the game as it stands into `view`, through its own camera, lit as
    /// the game is, and lays its screen out as drawn.
    ///
    /// # Errors
    /// A stylesheet that will not apply, a component that will not extract, or
    /// a frame that will not encode.
    pub fn draw(
        &mut self,
        renderer: &mut SceneRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
    ) -> Result<(), String> {
        let seconds = self.seconds;
        renderer.set_lit(true);
        self.run.draw_with(|extractor, world, session| {
            let sizes = renderer.draw_shot(
                device,
                queue,
                view,
                Shot {
                    // The run's own: it carries the project's render materials.
                    extractor,
                    world,
                    camera: CameraView::default(),
                    animations: Some(session.animations()),
                    effects: Some(session.effects()),
                    seconds,
                },
            )?;
            Ok(((), sizes))
        })
    }
}
