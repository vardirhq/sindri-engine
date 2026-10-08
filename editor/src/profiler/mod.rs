//! Where an editor frame's time goes.
//!
//! A frame is everything between one call to the editor's `ui` and the next:
//! the upkeep before anything moves, the fixed steps Play ran — physics, the
//! screen UI, effects, scripts, animation, cameras, placement — the views'
//! presentation, extraction and encoding, the panels laid out around them,
//! egui painting the window, and the wait until the next frame begins. Each is
//! timed on the CPU and kept for the last few seconds, with each script's
//! share of the scripts phase beside it, so "the game hitches when the boss
//! spawns" can become "the boss's script takes 9 ms on the step it spawns",
//! and "the editor is slow" can become "the hierarchy takes 6 ms a frame".
//!
//! Three of the phases are not timed directly but worked out when the frame
//! closes: [`Phase::Panels`] is what the editor's own `ui` took less every
//! phase measured inside it, [`Phase::Paint`] is the CPU time eframe reports
//! for the frame less the editor's own, and [`Phase::Waiting`] is the rest of
//! the interval — presenting, vsync and idling. eframe reports a frame's CPU
//! time during the next one, which is why a frame is closed by the start of
//! the one after it rather than by its own end.
//!
//! CPU time only. A view's encoding is the work of recording and submitting
//! its commands, not how long the GPU spends on them — except that a GPU still
//! busy with earlier frames makes a submission block, and that wait lands in
//! encoding. A benchmark asks each view to wait for the GPU instead, timed as
//! [`Phase::Gpu`], so its encoding is the CPU's alone.
//!
//! Kept apart from the drawing so what is recorded, and what is averaged out
//! of it, is a question a test can ask without a window.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use sindri_decay::ScriptTiming;

#[cfg(test)]
mod tests;

/// How many frames are kept: about five seconds at 60 frames a second.
pub const KEPT: usize = 300;

/// How many phases a frame is divided into.
pub const PHASES: usize = 15;

/// One part of a frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Phase {
    /// Hot reload, textures and scripts arriving, compiling: what the editor
    /// does before a frame's steps, whether or not anything plays.
    Upkeep,
    Effects,
    Physics,
    ScreenUi,
    Scripts,
    Animation,
    Cameras,
    /// Grid placements settling and any scene change a script asked for.
    Placement,
    /// Working out the world a view shows when it is not the world itself:
    /// Weave's presentation of it, or the Timeline's pose.
    Presentation,
    /// Measuring text and extracting a view's frame from the world.
    Extraction,
    /// Recording a view's GPU commands and submitting them.
    Encoding,
    /// Waiting for a view's GPU work to finish, which only a benchmark does
    /// so that the GPU's time is not counted as the next submission's.
    Gpu,
    /// Laying out every panel and its chrome: the editor's own `ui` less
    /// everything above.
    Panels,
    /// egui tessellating and painting the window, as eframe reports it.
    Paint,
    /// Presenting, vsync and idling until the next frame: time the editor did
    /// not spend.
    Waiting,
}

impl Phase {
    /// Every phase, in the order a frame runs them.
    pub const ALL: [Self; PHASES] = [
        Self::Upkeep,
        Self::Effects,
        Self::Physics,
        Self::ScreenUi,
        Self::Scripts,
        Self::Animation,
        Self::Cameras,
        Self::Placement,
        Self::Presentation,
        Self::Extraction,
        Self::Encoding,
        Self::Gpu,
        Self::Panels,
        Self::Paint,
        Self::Waiting,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Upkeep => "Upkeep",
            Self::Effects => "Effects",
            Self::Physics => "Physics",
            Self::ScreenUi => "Screen UI",
            Self::Scripts => "Scripts",
            Self::Animation => "Animation",
            Self::Cameras => "Cameras",
            Self::Placement => "Placement",
            Self::Presentation => "Presentation",
            Self::Extraction => "Extraction",
            Self::Encoding => "Encoding",
            Self::Gpu => "GPU",
            Self::Panels => "Panels",
            Self::Paint => "Paint",
            Self::Waiting => "Waiting",
        }
    }

    /// The phase's name in a benchmark report: stable, and not prose.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Upkeep => "upkeep",
            Self::Effects => "effects",
            Self::Physics => "physics",
            Self::ScreenUi => "screen_ui",
            Self::Scripts => "scripts",
            Self::Animation => "animation",
            Self::Cameras => "cameras",
            Self::Placement => "placement",
            Self::Presentation => "presentation",
            Self::Extraction => "extraction",
            Self::Encoding => "encoding",
            Self::Gpu => "gpu",
            Self::Panels => "panels",
            Self::Paint => "paint",
            Self::Waiting => "waiting",
        }
    }

    /// Whether the phase is the editor working, rather than waiting for the
    /// GPU, the display or something to do.
    pub const fn is_work(self) -> bool {
        !matches!(self, Self::Gpu | Self::Waiting)
    }

    /// Whether the phase is worked out at the frame's close rather than
    /// timed where it happens.
    const fn is_derived(self) -> bool {
        matches!(self, Self::Panels | Self::Paint | Self::Waiting)
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// One frame, timed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    /// Time spent in each phase, indexed as [`Phase::ALL`].
    pub phases: [Duration; PHASES],
    /// How many fixed steps the frame ran. Zero on a frame drawn between
    /// steps or while editing, two or more on one that caught up.
    pub steps: u32,
    /// Each script's time, summed over the frame's steps.
    pub scripts: Vec<ScriptTiming>,
}

impl Frame {
    pub fn phase(&self, phase: Phase) -> Duration {
        self.phases[phase.index()]
    }

    /// The whole interval the frame covered, waiting included.
    pub fn total(&self) -> Duration {
        self.phases.iter().sum()
    }

    /// The time the editor spent working in the frame: everything but
    /// [`Phase::Waiting`].
    pub fn work(&self) -> Duration {
        Phase::ALL
            .iter()
            .filter(|phase| phase.is_work())
            .map(|phase| self.phase(*phase))
            .sum()
    }

    /// What was timed where it happened, inside the editor's own `ui`.
    fn measured(&self) -> Duration {
        Phase::ALL
            .iter()
            .filter(|phase| !phase.is_derived())
            .map(|phase| self.phase(*phase))
            .sum()
    }
}

/// The frame being recorded, until the next one begins and closes it.
#[derive(Clone, Copy, Debug)]
struct Open {
    started: Instant,
    /// Whether the frame is to be kept even if nothing stepped in it.
    recording: bool,
    /// How long the editor's own `ui` took, once it has returned.
    drawn: Option<Duration>,
}

/// The frames recorded so far, the newest last.
#[derive(Debug, Default)]
pub struct Profiler {
    frames: VecDeque<Frame>,
    /// What the open frame has measured so far.
    current: Frame,
    open: Option<Open>,
    /// Whether frames in which nothing plays are recorded too, so the cost of
    /// the editor at rest can be read as well as the cost of a run.
    while_editing: bool,
    /// Every frame closed while capturing, without the [`KEPT`] limit: what a
    /// benchmark reads.
    captured: Option<Vec<Frame>>,
    /// Whether each view waits for its GPU work before the frame goes on.
    waits_for_gpu: bool,
}

impl Profiler {
    /// Closes the frame before this one and opens the next.
    ///
    /// `previous_cpu` is what eframe says the frame before took, which it
    /// reports during this one. `playing` is whether a run is going, which
    /// decides, with [`Self::set_while_editing`], whether this frame is kept.
    pub fn begin(&mut self, now: Instant, previous_cpu: Option<Duration>, playing: bool) {
        if let Some(open) = self.open.take() {
            let mut frame = std::mem::take(&mut self.current);
            if open.recording || frame.steps > 0 {
                close(&mut frame, open, now, previous_cpu);
                self.keep(frame);
            }
        }
        self.current = Frame::default();
        self.open = Some(Open {
            started: now,
            recording: playing || self.while_editing,
            drawn: None,
        });
    }

    /// Notes how long the editor's own `ui` took for the open frame.
    pub fn end(&mut self, drawn: Duration) {
        if let Some(open) = &mut self.open {
            open.drawn = Some(drawn);
        }
    }

    /// Adds time to a phase of the open frame.
    pub fn add(&mut self, phase: Phase, time: Duration) {
        self.current.phases[phase.index()] += time;
    }

    /// Counts a fixed step, with what each script took in it.
    pub fn step(&mut self, scripts: Vec<ScriptTiming>) {
        self.current.steps += 1;
        for timing in scripts {
            merge(&mut self.current.scripts, &timing);
        }
    }

    /// Forgets every frame, as starting Play again does.
    pub fn clear(&mut self) {
        self.frames.clear();
        self.current = Frame::default();
        self.open = None;
    }

    /// Whether views wait for the GPU after submitting, as a benchmark asks.
    pub const fn waits_for_gpu(&self) -> bool {
        self.waits_for_gpu
    }

    /// Makes each view wait for its GPU work, timing the wait as
    /// [`Phase::Gpu`]. Without it, a GPU still busy with the last frame makes
    /// the next submission block, and that wait is timed as encoding.
    pub fn set_waits_for_gpu(&mut self, on: bool) {
        self.waits_for_gpu = on;
    }

    pub const fn while_editing(&self) -> bool {
        self.while_editing
    }

    /// Records frames in which nothing plays, or stops recording them.
    pub fn set_while_editing(&mut self, on: bool) {
        self.while_editing = on;
    }

    /// Starts keeping every closed frame, however many, until
    /// [`Self::take_captured`].
    pub fn capture(&mut self) {
        self.captured = Some(Vec::new());
    }

    /// The frames closed since [`Self::capture`], and stops capturing.
    pub fn take_captured(&mut self) -> Vec<Frame> {
        self.captured.take().unwrap_or_default()
    }

    /// How many frames have been captured so far.
    pub fn captured_len(&self) -> usize {
        self.captured.as_ref().map_or(0, Vec::len)
    }

    pub fn frames(&self) -> &VecDeque<Frame> {
        &self.frames
    }

    /// What the kept frames add up to.
    pub fn summary(&self) -> Summary {
        Summary::of(self.frames.iter())
    }

    fn keep(&mut self, frame: Frame) {
        if let Some(captured) = &mut self.captured {
            captured.push(frame.clone());
        }
        if self.frames.len() == KEPT {
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
    }
}

/// Fills in the phases a frame can only know once the next has begun.
fn close(frame: &mut Frame, open: Open, now: Instant, previous_cpu: Option<Duration>) {
    let interval = now.saturating_duration_since(open.started);
    let Some(drawn) = open.drawn else {
        // A frame whose `ui` never returned is not one to explain.
        return;
    };
    frame.phases[Phase::Panels.index()] = drawn.saturating_sub(frame.measured());
    // eframe's figure covers the editor's `ui` too, so painting is what is
    // left of it. A frame it did not report is all the editor's own.
    let busy = previous_cpu.map_or(drawn, |cpu| cpu.max(drawn));
    frame.phases[Phase::Paint.index()] = busy.saturating_sub(drawn);
    frame.phases[Phase::Waiting.index()] = interval.saturating_sub(busy);
}

/// Adds one script's time to a list, under its existing entry if it has one.
fn merge(into: &mut Vec<ScriptTiming>, timing: &ScriptTiming) {
    match into
        .iter_mut()
        .find(|kept| kept.source == timing.source && kept.script == timing.script)
    {
        Some(kept) => {
            kept.runs += timing.runs;
            kept.time += timing.time;
        }
        None => into.push(timing.clone()),
    }
}

/// The kept frames, averaged and at their worst.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Summary {
    pub frames: usize,
    /// The average time the editor worked in a frame.
    pub average: Duration,
    /// The longest any frame's work took.
    pub worst: Duration,
    /// The average interval between frames, waiting included.
    pub interval: Duration,
    /// Each phase's average, indexed as [`Phase::ALL`].
    pub phases: [Duration; PHASES],
    /// Each script's average time and runs a frame, slowest first.
    pub scripts: Vec<ScriptTiming>,
}

impl Summary {
    pub fn of<'a>(frames: impl Iterator<Item = &'a Frame>) -> Self {
        let mut summary = Self::default();
        let mut work = Duration::ZERO;
        let mut interval = Duration::ZERO;
        let mut phases = [Duration::ZERO; PHASES];
        for frame in frames {
            summary.frames += 1;
            let time = frame.work();
            work += time;
            interval += frame.total();
            summary.worst = summary.worst.max(time);
            for (sum, phase) in phases.iter_mut().zip(frame.phases) {
                *sum += phase;
            }
            for timing in &frame.scripts {
                merge(&mut summary.scripts, timing);
            }
        }
        let Ok(count) = u32::try_from(summary.frames) else {
            return summary;
        };
        if count == 0 {
            return summary;
        }
        summary.average = work / count;
        summary.interval = interval / count;
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
