//! Driving the editor through a benchmark: wait for the project, record it at
//! rest, press Play, record the run, write the report, close.
//!
//! What a benchmark is asked for and what it writes are
//! [`crate::benchmark`]'s; this is only the sequence, which needs the app.

use eframe::egui;

use crate::benchmark::{BenchmarkPlan, report_json};
use crate::dock::Panel;
use crate::profiler::Frame;

use super::EditorApp;

/// The longest a benchmark waits for a project's assets before measuring it
/// anyway, in frames. A project that never finishes loading is still worth a
/// report, and one that says so.
const LOADING_LIMIT: usize = 1_800;

/// Where a benchmark has got to.
#[derive(Debug)]
enum Stage {
    Loading {
        waited: usize,
    },
    /// Leaving the editor untouched and asking for nothing, counting the
    /// frames it draws anyway. An editor at rest should draw none.
    Resting {
        /// When the rest began: the first resting frame, which the last
        /// loading frame asked for and so is not counted.
        began: Option<std::time::Instant>,
        frames: usize,
    },
    /// Letting frames pass before recording. `editing` is what the editing
    /// section recorded, once it has: settling with it is settling into Play.
    Settling {
        left: usize,
        editing: Option<Vec<Frame>>,
    },
    Editing,
    Playing {
        editing: Vec<Frame>,
    },
    Closing,
}

/// How long an editor at rest is watched.
const REST: std::time::Duration = std::time::Duration::from_secs(3);

/// How long it is given first to finish what opening started — the last
/// textures arriving, the first picture for the Scenes panel — whose frames
/// are not the editor at rest.
const GRACE: std::time::Duration = std::time::Duration::from_secs(1);

/// A benchmark in progress.
#[derive(Debug)]
pub(super) struct BenchmarkRun {
    plan: BenchmarkPlan,
    stage: Stage,
    /// What was opened, as the command line said it.
    opened: String,
    /// Frames drawn while the editor was left at rest.
    rest_frames: Option<usize>,
}

impl BenchmarkRun {
    pub(super) const fn new(plan: BenchmarkPlan, opened: String) -> Self {
        Self {
            plan,
            stage: Stage::Loading { waited: 0 },
            opened,
            rest_frames: None,
        }
    }
}

impl EditorApp {
    /// Moves a benchmark on by one frame, when one is running.
    pub(super) fn drive_benchmark(&mut self, context: &egui::Context) {
        let Some(mut run) = self.benchmark.take() else {
            return;
        };
        // Every frame is drawn while measuring: a benchmark is the cost of a
        // frame, and an editor that skipped frames would report none. Except
        // at rest, which is measuring whether it draws any.
        if !matches!(run.stage, Stage::Resting { .. }) {
            context.request_repaint();
        }
        run.stage = match std::mem::replace(&mut run.stage, Stage::Closing) {
            Stage::Loading { waited } => {
                let loading = self.scripts.loading() || self.textures.loading();
                if loading && waited < LOADING_LIMIT {
                    Stage::Loading { waited: waited + 1 }
                } else {
                    if loading {
                        self.console
                            .error("Benchmark: assets were still loading when measuring began");
                    }
                    // Each view's GPU work is waited for and timed apart,
                    // as the standalone benchmark does, so the two compare
                    // the CPU's work rather than how far behind the GPU is.
                    self.profiler.set_waits_for_gpu(true);
                    Stage::Resting {
                        began: None,
                        frames: 0,
                    }
                }
            }
            // Woken when the rest is over; anything else that draws a frame
            // meanwhile is the editor not being idle.
            Stage::Resting {
                began: None,
                frames,
            } => {
                // From another thread, as the disk watcher wakes the editor:
                // a timed repaint asked for inside a frame is not kept by
                // every platform once the window has gone quiet.
                let waking = context.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(GRACE + REST);
                    waking.request_repaint();
                });
                Stage::Resting {
                    began: Some(std::time::Instant::now()),
                    frames,
                }
            }
            Stage::Resting {
                began: Some(began),
                frames,
            } if began.elapsed() < GRACE + REST => Stage::Resting {
                began: Some(began),
                frames: frames + usize::from(began.elapsed() >= GRACE),
            },
            Stage::Resting { frames, .. } => {
                run.rest_frames = Some(frames);
                Stage::Settling {
                    left: run.plan.settle,
                    editing: None,
                }
            }
            Stage::Settling { left, editing } if left > 0 => Stage::Settling {
                left: left - 1,
                editing,
            },
            Stage::Settling { editing, .. } => {
                self.profiler.capture();
                if let Some(editing) = editing {
                    Stage::Playing { editing }
                } else {
                    self.profiler.set_while_editing(true);
                    Stage::Editing
                }
            }
            Stage::Editing if self.profiler.captured_len() < run.plan.frames => Stage::Editing,
            Stage::Editing => {
                let editing = self.profiler.take_captured();
                self.profiler.set_while_editing(false);
                self.preferences.workspace.reveal(Panel::Game);
                self.toggle_play_mode();
                Stage::Settling {
                    left: run.plan.settle,
                    editing: Some(editing),
                }
            }
            Stage::Playing { editing } if self.profiler.captured_len() < run.plan.frames => {
                Stage::Playing { editing }
            }
            Stage::Playing { editing } => {
                let playing = self.profiler.take_captured();
                let errors: Vec<String> = self
                    .console
                    .at_least(crate::console::Level::Error)
                    .map(|entry| format!("{} (x{})", entry.message, entry.count))
                    .collect();
                write_report(&run, context, &editing, &playing, &errors);
                self.toggle_play_mode();
                Stage::Closing
            }
            Stage::Closing => {
                context.send_viewport_cmd(egui::ViewportCommand::Close);
                Stage::Closing
            }
        };
        self.benchmark = Some(run);
    }
}

/// Writes what was recorded, and says where, or why it could not.
fn write_report(
    run: &BenchmarkRun,
    context: &egui::Context,
    editing: &[Frame],
    playing: &[Frame],
    errors: &[String],
) {
    let window = context.content_rect().size();
    let rest = run.rest_frames.map(|frames| (REST.as_secs_f32(), frames));
    let report = report_json(
        &run.opened,
        [window.x, window.y],
        rest,
        editing,
        playing,
        errors,
    );
    let written = serde_json::to_string_pretty(&report)
        .map_err(|error| error.to_string())
        .and_then(|text| std::fs::write(&run.plan.report, text).map_err(|error| error.to_string()));
    match written {
        Ok(()) => println!(
            "Benchmark: {} editing and {} playing frames written to {}",
            editing.len(),
            playing.len(),
            run.plan.report.display()
        ),
        Err(error) => eprintln!(
            "Benchmark: could not write {}: {error}",
            run.plan.report.display()
        ),
    }
}
