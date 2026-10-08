//! What one fixed step did, for a host that wants to show it.
//!
//! The shipped hosts log a step's printing and failures; the editor puts them
//! in its console against the entity that said them, and its Profiler wants
//! where the step's time went. One report serves both, so neither has to
//! reach into the session for what it did.

use std::time::Duration;

use sindri_decay::ScriptReport;

/// One part of a fixed step, in the order a step runs them.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum StepPhase {
    /// Both solvers advancing, so the scripts see this step's events.
    Physics,
    /// Laying out the screen UI and reading the pointer's presses against it.
    ScreenUi,
    /// The flecks scripts have thrown moving on.
    Effects,
    /// Gestures, aim and every script's tick.
    Scripts,
    /// Sprite animations and sequences moving with gameplay.
    Animation,
    /// Cameras following where the step left what they follow.
    Cameras,
    /// Grid placements settling and any scene change a script asked for.
    Placement,
}

impl StepPhase {
    pub const ALL: [Self; 7] = [
        Self::Physics,
        Self::ScreenUi,
        Self::Effects,
        Self::Scripts,
        Self::Animation,
        Self::Cameras,
        Self::Placement,
    ];

    const fn index(self) -> usize {
        self as usize
    }
}

/// How long each phase of a step took, when the session was asked to measure.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StepTimes([Duration; StepPhase::ALL.len()]);

impl StepTimes {
    #[must_use]
    pub fn phase(&self, phase: StepPhase) -> Duration {
        self.0[phase.index()]
    }

    pub(crate) fn add(&mut self, phase: StepPhase, time: Duration) {
        self.0[phase.index()] += time;
    }

    /// The whole step.
    #[must_use]
    pub fn total(&self) -> Duration {
        self.0.iter().sum()
    }
}

/// What one fixed step did.
#[derive(Debug, Default)]
pub struct StepReport {
    /// What the scripts printed and what failed in them, and each script's
    /// time when measuring.
    pub scripts: ScriptReport,
    /// Problems that did not stop the step: a sequence cue that named
    /// nothing, a grid placement that could not settle. Each is already
    /// worded for a person.
    pub problems: Vec<String>,
    /// Where the step's time went. Zero unless the session is measuring.
    pub times: StepTimes,
}

impl StepReport {
    /// Writes what the step said to the log: prints at info, failures and
    /// problems as errors. What a shipped host does with a step, which has
    /// nowhere else to put it.
    pub fn log(&self) {
        for failure in &self.scripts.failures {
            log::error!("{failure}");
        }
        for message in &self.scripts.printed {
            log::info!("{}", message.message);
        }
        for problem in &self.problems {
            log::error!("{problem}");
        }
    }
}

/// Times consecutive phases of a step, each lap ending one and starting the
/// next. Reads no clock unless measuring, because a browser's clock is not
/// free and a shipped game never asks.
pub(crate) struct Laps {
    last: Option<web_time::Instant>,
    times: StepTimes,
}

impl Laps {
    pub(crate) fn start(measuring: bool) -> Self {
        Self {
            last: measuring.then(web_time::Instant::now),
            times: StepTimes::default(),
        }
    }

    pub(crate) fn lap(&mut self, phase: StepPhase) {
        if let Some(last) = &mut self.last {
            let now = web_time::Instant::now();
            self.times.add(phase, now - *last);
            *last = now;
        }
    }

    pub(crate) const fn finish(self) -> StepTimes {
        self.times
    }
}
