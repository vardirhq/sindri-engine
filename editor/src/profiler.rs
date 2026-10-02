//! Where Play's time goes, frame by frame.
//!
//! A frame of Play is the fixed steps it ran — effects, physics, the screen
//! UI, scripts, sprite animation, cameras — and the views it drew. Each is
//! timed on the CPU while Play runs and kept for the last few seconds, with
//! each script's share of the scripts phase beside it, so "the game hitches
//! when the boss spawns" can become "the boss's script takes 9 ms on the
//! step it spawns".
//!
//! CPU time only. A view's time is the work of preparing and submitting its
//! frame, not how long the GPU spends drawing it.
//!
//! Kept apart from the drawing so what is recorded, and what is averaged out
//! of it, is a question a test can ask without a window.

use std::collections::VecDeque;
use std::time::Duration;

use sindri_decay::ScriptTiming;

/// How many frames are kept: about five seconds at 60 frames a second.
pub const KEPT: usize = 300;

/// One part of a frame of Play.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Phase {
    Effects,
    Physics,
    ScreenUi,
    Scripts,
    Animation,
    Cameras,
    SceneView,
    GameView,
}

impl Phase {
    /// Every phase, in the order a frame runs them.
    pub const ALL: [Self; 8] = [
        Self::Effects,
        Self::Physics,
        Self::ScreenUi,
        Self::Scripts,
        Self::Animation,
        Self::Cameras,
        Self::SceneView,
        Self::GameView,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Effects => "Effects",
            Self::Physics => "Physics",
            Self::ScreenUi => "Screen UI",
            Self::Scripts => "Scripts",
            Self::Animation => "Animation",
            Self::Cameras => "Cameras",
            Self::SceneView => "Scene view",
            Self::GameView => "Game view",
        }
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// One frame of Play, timed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    /// Time spent in each phase, indexed as [`Phase::ALL`].
    pub phases: [Duration; 8],
    /// How many fixed steps the frame ran. Zero on a frame drawn between
    /// steps, two or more on one that caught up.
    pub steps: u32,
    /// Each script's time, summed over the frame's steps.
    pub scripts: Vec<ScriptTiming>,
}

impl Frame {
    pub fn phase(&self, phase: Phase) -> Duration {
        self.phases[phase.index()]
    }

    /// Everything measured in the frame.
    pub fn total(&self) -> Duration {
        self.phases.iter().sum()
    }
}

/// The frames of Play recorded so far, the newest last.
#[derive(Debug, Default)]
pub struct Profiler {
    frames: VecDeque<Frame>,
    /// The frame being recorded, until [`Profiler::finish`] keeps it.
    current: Frame,
    /// Set by anything timed this frame, so a frame in which Play did nothing
    /// — paused, or stopped — is not kept as a frame that cost nothing.
    recorded: bool,
}

impl Profiler {
    /// Adds time to a phase of the frame being recorded.
    pub fn add(&mut self, phase: Phase, time: Duration) {
        self.current.phases[phase.index()] += time;
        self.recorded = true;
    }

    /// Counts a fixed step, with what each script took in it.
    pub fn step(&mut self, scripts: Vec<ScriptTiming>) {
        self.current.steps += 1;
        self.recorded = true;
        for timing in scripts {
            match self
                .current
                .scripts
                .iter_mut()
                .find(|kept| kept.source == timing.source && kept.script == timing.script)
            {
                Some(kept) => {
                    kept.runs += timing.runs;
                    kept.time += timing.time;
                }
                None => self.current.scripts.push(timing),
            }
        }
    }

    /// Ends the frame: keeps it if Play did anything in it, dropping the
    /// oldest once [`KEPT`] are held.
    pub fn finish(&mut self) {
        let frame = std::mem::take(&mut self.current);
        if !std::mem::take(&mut self.recorded) {
            return;
        }
        if self.frames.len() == KEPT {
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
    }

    /// Forgets every frame, as starting Play again does.
    pub fn clear(&mut self) {
        self.frames.clear();
        self.current = Frame::default();
        self.recorded = false;
    }

    pub fn frames(&self) -> &VecDeque<Frame> {
        &self.frames
    }

    /// What the kept frames add up to.
    pub fn summary(&self) -> Summary {
        Summary::of(self.frames.iter())
    }
}

/// The kept frames, averaged and at their worst.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Summary {
    pub frames: usize,
    pub average: Duration,
    pub worst: Duration,
    /// Each phase's average, indexed as [`Phase::ALL`].
    pub phases: [Duration; 8],
    /// Each script's average time and runs a frame, slowest first.
    pub scripts: Vec<ScriptTiming>,
}

impl Summary {
    pub fn of<'a>(frames: impl Iterator<Item = &'a Frame>) -> Self {
        let mut summary = Self::default();
        let mut total = Duration::ZERO;
        let mut phases = [Duration::ZERO; 8];
        for frame in frames {
            summary.frames += 1;
            let time = frame.total();
            total += time;
            summary.worst = summary.worst.max(time);
            for (sum, phase) in phases.iter_mut().zip(frame.phases) {
                *sum += phase;
            }
            for timing in &frame.scripts {
                match summary
                    .scripts
                    .iter_mut()
                    .find(|kept| kept.source == timing.source && kept.script == timing.script)
                {
                    Some(kept) => {
                        kept.runs += timing.runs;
                        kept.time += timing.time;
                    }
                    None => summary.scripts.push(timing.clone()),
                }
            }
        }
        let Ok(count) = u32::try_from(summary.frames) else {
            return summary;
        };
        if count == 0 {
            return summary;
        }
        summary.average = total / count;
        summary.phases = phases.map(|sum| sum / count);
        for timing in &mut summary.scripts {
            timing.time /= count;
            timing.runs = (timing.runs + count / 2) / count;
        }
        summary
            .scripts
            .sort_by(|a, b| b.time.cmp(&a.time).then_with(|| a.script.cmp(&b.script)));
        summary
    }

    pub fn phase(&self, phase: Phase) -> Duration {
        self.phases[phase.index()]
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use sindri_decay::ScriptTiming;

    use super::{KEPT, Phase, Profiler};

    fn timing(script: &str, millis: u64) -> ScriptTiming {
        ScriptTiming {
            source: "game.decay".to_owned(),
            script: script.to_owned(),
            runs: 1,
            time: Duration::from_millis(millis),
        }
    }

    #[test]
    fn a_frame_adds_up_its_steps_and_a_frame_that_did_nothing_is_not_kept() {
        let mut profiler = Profiler::default();
        profiler.add(Phase::Physics, Duration::from_millis(1));
        profiler.step(vec![timing("Boss", 3)]);
        profiler.add(Phase::Physics, Duration::from_millis(2));
        profiler.step(vec![timing("Boss", 4), timing("Wisp", 1)]);
        profiler.finish();
        profiler.finish();
        assert_eq!(profiler.frames().len(), 1, "the empty frame is dropped");
        let frame = &profiler.frames()[0];
        assert_eq!(frame.steps, 2);
        assert_eq!(frame.phase(Phase::Physics), Duration::from_millis(3));
        assert_eq!(frame.scripts[0].runs, 2);
        assert_eq!(frame.scripts[0].time, Duration::from_millis(7));
    }

    #[test]
    fn the_summary_averages_and_ranks_the_slowest_script_first() {
        let mut profiler = Profiler::default();
        for (boss, wisp) in [(2, 5), (6, 5)] {
            profiler.add(Phase::Scripts, Duration::from_millis(boss + wisp));
            profiler.step(vec![timing("Boss", boss), timing("Wisp", wisp)]);
            profiler.finish();
        }
        let summary = profiler.summary();
        assert_eq!(summary.frames, 2);
        assert_eq!(summary.average, Duration::from_millis(9));
        assert_eq!(summary.worst, Duration::from_millis(11));
        assert_eq!(summary.phase(Phase::Scripts), Duration::from_millis(9));
        let ranked: Vec<&str> = summary.scripts.iter().map(|t| t.script.as_str()).collect();
        assert_eq!(ranked, ["Wisp", "Boss"]);
        assert_eq!(summary.scripts[1].time, Duration::from_millis(4));
        assert_eq!(summary.scripts[1].runs, 1, "runs a frame, not in all");
    }

    #[test]
    fn only_the_newest_frames_are_kept() {
        let mut profiler = Profiler::default();
        for _ in 0..KEPT + 5 {
            profiler.add(Phase::Effects, Duration::from_micros(1));
            profiler.finish();
        }
        assert_eq!(profiler.frames().len(), KEPT);
        profiler.clear();
        assert!(profiler.frames().is_empty());
    }
}
