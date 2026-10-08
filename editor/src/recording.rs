//! A run, recorded so it can be scrubbed.
//!
//! Every fixed step's input is kept, with whether the Game view was drawn
//! after it — a draw lays the screen out, and the next step's clicks are
//! hit-tested against that layout — and every [`EVERY`] steps a copy of the
//! world and the session. Going to a recorded step restores the copy at or
//! before it and steps forward with the recorded input, which a session
//! restored from a checkpoint does exactly as it did the first time
//! (`crates/sindri-runtime/tests/a_run_goes_back.rs`).
//!
//! **Memory is bounded by time.** At most [`KEEP`] copies are kept, one a
//! second at the editor's fixed rate, so a run can be scrubbed over its last
//! two minutes; older copies, and the input before the oldest, are let go.
//! What that costs is a world and a session per second of the window: for
//! Orbital Last Stand a few megabytes a copy.

use std::collections::VecDeque;

use sindri_core::World;
use sindri_platform::InputState;
use sindri_runtime::{Checkpoint, Session};

/// How many steps apart the copies are: a second at sixty steps a second.
pub const EVERY: u64 = 60;

/// How many copies are kept: two minutes of a run.
pub const KEEP: usize = 120;

/// A copy of the run at one step.
pub struct Mark {
    pub step: u64,
    pub world: World,
    pub session: Checkpoint,
}

/// What one step was given.
#[derive(Clone, Debug)]
pub struct RecordedStep {
    pub input: InputState,
    /// The screen the step's UI was laid out against, in points.
    pub viewport: (f32, f32),
    pub delta: f32,
    /// Whether the Game view was drawn after this step, and at what logical
    /// size: the draw that the next step's clicks were hit-tested against.
    pub drawn: Option<weave::Viewport>,
}

/// A run's recording.
pub struct Recording {
    marks: VecDeque<Mark>,
    /// Steps from `first` on, in order.
    steps: VecDeque<RecordedStep>,
    first: u64,
    /// The number of the next step to run.
    next: u64,
    /// Where scrubbing left the run, while it is somewhere in the past: the
    /// recorded future is kept until the run moves on from here.
    scrubbed: Option<u64>,
}

impl Recording {
    /// Starts recording a run whose world and session are as given.
    #[must_use]
    pub fn start(world: &World, session: &Session) -> Self {
        let mut marks = VecDeque::new();
        marks.push_back(Mark {
            step: 0,
            world: world.clone(),
            session: session.checkpoint(),
        });
        Self {
            marks,
            steps: VecDeque::new(),
            first: 0,
            next: 0,
            scrubbed: None,
        }
    }

    /// Notes what the next step is given, before it runs. A run that moves
    /// on from a scrubbed step lets the recorded future go.
    pub fn before_step(&mut self, step: RecordedStep) {
        if let Some(at) = self.scrubbed.take() {
            self.truncate(at);
        }
        self.steps.push_back(step);
    }

    /// Notes that the step ran, copying the run if it is time to.
    pub fn after_step(&mut self, world: &World, session: &Session) {
        self.next += 1;
        if self.next.is_multiple_of(EVERY) {
            self.marks.push_back(Mark {
                step: self.next,
                world: world.clone(),
                session: session.checkpoint(),
            });
            while self.marks.len() > KEEP {
                self.marks.pop_front();
            }
            let oldest = self.marks.front().map_or(0, |mark| mark.step);
            while self.first < oldest && !self.steps.is_empty() {
                self.steps.pop_front();
                self.first += 1;
            }
        }
    }

    /// Notes that the Game view drew the run after the latest step.
    pub fn drew(&mut self, viewport: weave::Viewport) {
        if self.scrubbed.is_none()
            && let Some(step) = self.steps.back_mut()
        {
            step.drawn = Some(viewport);
        }
    }

    /// The steps that can be gone to: from the oldest copy to the latest
    /// step recorded.
    #[must_use]
    pub fn range(&self) -> (u64, u64) {
        (self.marks.front().map_or(0, |mark| mark.step), self.next)
    }

    /// Where the run is: the scrubbed step, or the latest.
    #[must_use]
    pub fn at(&self) -> u64 {
        self.scrubbed.unwrap_or(self.next)
    }

    /// The copy to restore to reach `target`, and the steps to replay from
    /// it. `None` outside [`Self::range`].
    #[must_use]
    pub fn seek(&self, target: u64) -> Option<(&Mark, Vec<RecordedStep>)> {
        let (from, to) = self.range();
        if target < from || target > to {
            return None;
        }
        let mark = self.marks.iter().rev().find(|mark| mark.step <= target)?;
        let start = usize::try_from(mark.step - self.first).ok()?;
        let end = usize::try_from(target - self.first).ok()?;
        Some((mark, self.steps.range(start..end).cloned().collect()))
    }

    /// Records that the run now stands at `target`, which it was taken back
    /// (or forward) to.
    pub fn scrubbed_to(&mut self, target: u64) {
        self.scrubbed = (target != self.next).then_some(target);
    }

    /// Lets everything after `at` go: the run carries on from there.
    fn truncate(&mut self, at: u64) {
        while self.marks.back().is_some_and(|mark| mark.step > at) {
            self.marks.pop_back();
        }
        let keep = usize::try_from(at.saturating_sub(self.first)).unwrap_or(usize::MAX);
        self.steps.truncate(keep);
        self.next = at;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step() -> RecordedStep {
        RecordedStep {
            input: InputState::default(),
            viewport: (100.0, 100.0),
            delta: 1.0 / 60.0,
            drawn: None,
        }
    }

    fn session() -> Session {
        Session::with_sources(
            sindri_core::ComponentSchemaRegistry::default(),
            sindri_decay::ScriptSources::new(),
        )
    }

    #[test]
    fn copies_are_kept_for_the_window_and_the_input_with_them() {
        let world = World::default();
        let session = session();
        let mut recording = Recording::start(&world, &session);
        let total = EVERY * (KEEP as u64 + 10);
        for _ in 0..total {
            recording.before_step(step());
            recording.after_step(&world, &session);
        }
        let (from, to) = recording.range();
        assert_eq!(to, total);
        assert_eq!(
            to - from,
            EVERY * (KEEP as u64 - 1),
            "two minutes back, no more"
        );
        let (mark, replay) = recording.seek(from + EVERY + 7).expect("in range");
        assert_eq!(mark.step, from + EVERY);
        assert_eq!(replay.len(), 7);
        assert!(recording.seek(from - 1).is_none(), "older than the window");
    }

    #[test]
    fn moving_on_from_a_scrubbed_step_lets_the_future_go() {
        let world = World::default();
        let session = session();
        let mut recording = Recording::start(&world, &session);
        for _ in 0..200 {
            recording.before_step(step());
            recording.after_step(&world, &session);
        }
        recording.scrubbed_to(90);
        assert_eq!(recording.at(), 90);
        assert_eq!(recording.range().1, 200, "the future is kept while looking");
        recording.before_step(step());
        recording.after_step(&world, &session);
        assert_eq!(recording.range().1, 91, "and let go once the run moves on");
        let (mark, replay) = recording.seek(91).expect("in range");
        assert_eq!(mark.step, 60);
        assert_eq!(replay.len(), 31);
    }
}
