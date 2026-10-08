//! What a benchmark run of the editor is asked to do, and what it writes.
//!
//! `sindri-editor <scene or project> --benchmark <report.json>` opens the
//! scene the way a person would, waits for its assets, records frames of the
//! editor at rest, presses Play, records frames of the run, and writes every
//! frame's phases to the report before closing. `scripts/frame-benchmark.py`
//! runs it beside the standalone host and compares the two; this module is
//! only the editor's half, kept apart from the window so a test can read what
//! it parses and what it writes.
//!
//! A report is raw frames rather than averages, beside the console's errors, so what is made of them —
//! percentiles, ratios, the table in `docs/editor-update.md` — can change
//! without running anything again.

use std::path::PathBuf;

use serde_json::{Map, Value, json};

use crate::profiler::{Frame, Phase};

/// How many frames each section records unless told otherwise.
pub const DEFAULT_FRAMES: usize = 600;
/// How many frames are let pass before a section starts recording.
pub const DEFAULT_SETTLE: usize = 60;

/// A benchmark the command line asked for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BenchmarkPlan {
    /// Where the report is written.
    pub report: PathBuf,
    /// How many frames each section records.
    pub frames: usize,
    /// How many frames pass before each section records, so loading and the
    /// first run's warm-up are not measured as steady state.
    pub settle: usize,
}

impl BenchmarkPlan {
    /// Reads `--benchmark <report> [--frames N] [--settle N]` from the
    /// arguments after the path being opened. `None` when no benchmark is
    /// asked for; an error names what was malformed.
    pub fn from_args(args: &[String]) -> Result<Option<Self>, String> {
        let mut report = None;
        let mut frames = DEFAULT_FRAMES;
        let mut settle = DEFAULT_SETTLE;
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            let mut value = |name: &str| {
                rest.next()
                    .cloned()
                    .ok_or_else(|| format!("{name} needs a value"))
            };
            match arg.as_str() {
                "--benchmark" => report = Some(PathBuf::from(value("--benchmark")?)),
                "--frames" => frames = count(&value("--frames")?, "--frames")?,
                "--settle" => settle = count(&value("--settle")?, "--settle")?,
                other => return Err(format!("unknown argument {other}")),
            }
        }
        let Some(report) = report else {
            return if args.is_empty() {
                Ok(None)
            } else {
                Err("--frames and --settle only mean something with --benchmark".to_owned())
            };
        };
        if frames == 0 {
            return Err("--frames must be at least 1".to_owned());
        }
        Ok(Some(Self {
            report,
            frames,
            settle,
        }))
    }
}

fn count(text: &str, name: &str) -> Result<usize, String> {
    text.parse()
        .map_err(|_| format!("{name} wants a whole number, not {text:?}"))
}

/// One frame as a report holds it: its steps, and each phase in microseconds.
pub fn frame_json(frame: &Frame) -> Value {
    let phases: Map<String, Value> = Phase::ALL
        .iter()
        .map(|phase| {
            let micros = u64::try_from(frame.phase(*phase).as_micros()).unwrap_or(u64::MAX);
            (phase.key().to_owned(), Value::from(micros))
        })
        .collect();
    json!({ "steps": frame.steps, "phases": phases })
}

/// A whole report: what was opened, how the editor was built, and each
/// section's frames.
pub fn report_json(
    opened: &str,
    window: [f32; 2],
    rest: Option<(f32, usize)>,
    editing: &[Frame],
    playing: &[Frame],
    errors: &[String],
) -> Value {
    json!({
        "host": "editor",
        "opened": opened,
        "optimized": !cfg!(debug_assertions),
        "window": window,
        // Frames drawn while the editor was left untouched for so many
        // seconds before measuring: an editor at rest should draw none.
        "rest": rest.map(|(seconds, frames)| json!({ "seconds": seconds, "frames": frames })),
        // What the console held as errors when the run ended: a run that
        // measured fast because its scripts failed should say so.
        "errors": errors,
        "sections": {
            "editing": editing.iter().map(frame_json).collect::<Vec<_>>(),
            "playing": playing.iter().map(frame_json).collect::<Vec<_>>(),
        },
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{BenchmarkPlan, DEFAULT_SETTLE, frame_json, report_json};
    use crate::profiler::{Frame, Phase};

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_owned()).collect()
    }

    #[test]
    fn no_arguments_is_no_benchmark() {
        assert_eq!(BenchmarkPlan::from_args(&[]), Ok(None));
    }

    #[test]
    fn a_benchmark_reads_its_report_and_counts() {
        let plan = BenchmarkPlan::from_args(&args(&["--benchmark", "out.json", "--frames", "90"]))
            .unwrap()
            .unwrap();
        assert_eq!(plan.report.to_str(), Some("out.json"));
        assert_eq!(plan.frames, 90);
        assert_eq!(plan.settle, DEFAULT_SETTLE);
    }

    #[test]
    fn malformed_arguments_say_what_is_wrong() {
        for (words, said) in [
            (&["--benchmark"][..], "needs a value"),
            (
                &["--benchmark", "a.json", "--frames", "lots"],
                "whole number",
            ),
            (&["--benchmark", "a.json", "--frames", "0"], "at least 1"),
            (&["--frames", "10"], "only mean something"),
            (&["--fast"], "unknown argument"),
        ] {
            let error = BenchmarkPlan::from_args(&args(words)).unwrap_err();
            assert!(error.contains(said), "{words:?}: {error}");
        }
    }

    #[test]
    fn a_report_holds_every_phase_of_every_frame_in_microseconds() {
        let mut frame = Frame {
            steps: 2,
            ..Frame::default()
        };
        frame.phases[1] = Duration::from_micros(1500);
        let written = frame_json(&frame);
        assert_eq!(written["steps"], 2);
        assert_eq!(written["phases"][Phase::Effects.key()], 1500);
        assert_eq!(
            written["phases"].as_object().unwrap().len(),
            Phase::ALL.len()
        );
        let errors = ["Hero: no such field".to_owned()];
        let report = report_json(
            "games/platformer",
            [1440.0, 1024.0],
            Some((3.0, 0)),
            &[],
            &[frame],
            &errors,
        );
        assert_eq!(report["rest"]["frames"], 0);
        assert_eq!(report["host"], "editor");
        assert_eq!(report["sections"]["playing"].as_array().unwrap().len(), 1);
        assert!(report["sections"]["editing"].as_array().unwrap().is_empty());
        assert_eq!(report["errors"][0], "Hero: no such field");
    }
}
