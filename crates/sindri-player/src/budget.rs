//! Notices a device that cannot keep up, so the player can ask less of it.
//!
//! A frame's real length, smoothed, is the only thing every device reports the same way: a
//! phone that takes 40 ms to draw a lit scene says so by its frames being that long, whether
//! the time goes to the CPU or the GPU. When the smoothed length stays over the budget for a
//! while the player lowers [`sindri_scene::LightingQuality`]. It does not go back up: a
//! device that was struggling would only struggle again, and a frame rate that flickers
//! between two looks is worse than either.

use std::time::Duration;

use sindri_scene::{LightingQuality, SceneExtractor};

/// A frame longer than this, smoothed, is a device that is not keeping up: about 40 frames
/// a second.
const SLOW: f32 = 0.025;
/// Real seconds, and at least this many frames, ignored while the page loads and settles:
/// first draws compile pipelines. Counted in time and not only in frames, because the
/// devices that most need this draw a frame every few hundred milliseconds.
const SETTLE_SECONDS: f32 = 3.0;
const SETTLE_FRAMES: u32 = 12;
/// How long, in real seconds, the smoothed length must stay over `SLOW`.
const SUSTAIN_SECONDS: f32 = 1.5;
/// A frame counted as no longer than this in the smoothing, so a tab switch or a hitch is not
/// taken for the device's pace.
const LONGEST: f32 = 0.1;

#[derive(Debug, Default)]
pub(crate) struct FrameBudget {
    frames: u32,
    elapsed: f32,
    smoothed: f32,
    over: f32,
    spent: bool,
}

impl FrameBudget {
    /// Lowers the lighting of `scene` the frame the device is judged unable to keep up.
    pub(crate) fn keep_up(&mut self, scene: &mut SceneExtractor, delta: Duration, counts: bool) {
        if self.observe(delta, counts) {
            scene.set_lighting_quality(LightingQuality::Reduced);
        }
    }

    /// Takes one drawn frame's real length. Returns `true` once, the frame the device is
    /// judged unable to keep up; `counts` is false while the game is paused or hidden, when
    /// a frame says nothing of the device.
    pub(crate) fn observe(&mut self, delta: Duration, counts: bool) -> bool {
        if self.spent || !counts {
            return false;
        }
        let real = delta.as_secs_f32();
        self.frames += 1;
        self.elapsed += real;
        if self.elapsed < SETTLE_SECONDS || self.frames < SETTLE_FRAMES {
            return false;
        }
        let seconds = real.min(LONGEST);
        self.smoothed = if self.smoothed == 0.0 {
            seconds
        } else {
            self.smoothed.mul_add(0.9, seconds * 0.1)
        };
        if self.smoothed > SLOW {
            self.over += real.min(LONGEST);
        } else {
            self.over = 0.0;
        }
        if self.over >= SUSTAIN_SECONDS {
            self.spent = true;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(budget: &mut FrameBudget, frames: u32, millis: u64) -> bool {
        (0..frames).any(|_| budget.observe(Duration::from_millis(millis), true))
    }

    #[test]
    fn a_device_at_sixty_frames_a_second_is_left_alone() {
        let mut budget = FrameBudget::default();
        assert!(!run(&mut budget, 2000, 16));
    }

    #[test]
    fn a_device_at_twenty_five_frames_a_second_is_lowered_once() {
        let mut budget = FrameBudget::default();
        assert!(run(&mut budget, 600, 40));
        assert!(!run(&mut budget, 600, 40), "only the once");
    }

    #[test]
    fn a_device_at_three_frames_a_second_is_lowered_within_seconds() {
        let mut budget = FrameBudget::default();
        let frames = (0..200).find(|_| budget.observe(Duration::from_millis(330), true));
        assert!(
            frames.is_some_and(|at| at < 40),
            "caught after {frames:?} frames, about 13 s at that pace"
        );
    }

    #[test]
    fn the_loading_frames_and_a_hitch_do_not_count() {
        let mut budget = FrameBudget::default();
        assert!(
            !budget.observe(Duration::from_millis(1500), true),
            "a first draw"
        );
        assert!(
            !budget.observe(Duration::from_millis(1400), true),
            "pipelines"
        );
        assert!(!run(&mut budget, 1000, 16));
        assert!(
            !budget.observe(Duration::from_secs(3), true),
            "one long frame"
        );
        assert!(!run(&mut budget, 1000, 16));
    }

    #[test]
    fn a_paused_game_says_nothing_of_the_device() {
        let mut budget = FrameBudget::default();
        assert!((0..2000).all(|_| !budget.observe(Duration::from_millis(100), false)));
    }
}
