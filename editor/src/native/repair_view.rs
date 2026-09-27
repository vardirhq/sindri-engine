//! A script's compiler errors, and a fix the local model proposes for them.
//!
//! Drawn under a `.decay` preview. The errors come from the same check Play
//! compiles with, so what this says is what the scene would say, and it says it
//! whether or not any assistant is set up. The fix is offered only once a model
//! has proved on this machine that it can repair Decay, and it is exactly
//! that — an offer: a diff that already compiles, written to disk only when the
//! person accepts it, and put back with one button afterwards.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant};

use eframe::egui::{self, RichText};
use sindri_decay::{SourceCheck, check_source};

use crate::assistant::diff::{self, Line};
use crate::assistant::install::Shared;
use crate::assistant::repair::{Failure, MAX_REPAIRS, Proposal, repair};
use crate::preview::TextPreview;
use crate::ui::theme::{color, metric, text};
use crate::ui::widgets::{button, button::Intent, panel};

use super::EditorApp;
use super::assistant_view::Connector;

/// Unchanged lines shown around each change in a proposed diff.
const CONTEXT: usize = 2;

#[derive(Default)]
pub(crate) struct RepairState {
    /// The last check, and the file and text it was made against.
    checked: Option<(PathBuf, String, SourceCheck)>,
    asking: Option<Asking>,
    offer: Option<Offer>,
    /// What the last request came to, when it came to no offer.
    outcome: Option<(PathBuf, String)>,
    /// The text a fix replaced, so accepting it is never one-way.
    previous: Option<(PathBuf, String)>,
}

struct Asking {
    path: PathBuf,
    original: String,
    model: String,
    cancel: Arc<Shared>,
    answer: Receiver<Result<Proposal, Failure>>,
    since: Instant,
}

struct Offer {
    path: PathBuf,
    original: String,
    proposal: Proposal,
    diff: Vec<Line>,
}

impl EditorApp {
    /// Compiler errors for a previewed script, and the assistant's part in them.
    pub(super) fn script_problems(&mut self, ui: &mut egui::Ui, path: &Path, body: &str) {
        self.take_answer();
        let stale = self
            .assistant
            .repair
            .checked
            .as_ref()
            .is_none_or(|(checked, source, _)| checked != path || source != body);
        if stale {
            let check = check_source(body);
            self.assistant.repair.checked = Some((path.to_path_buf(), body.to_owned(), check));
        }
        let Some((_, _, check)) = &self.assistant.repair.checked else {
            return;
        };
        let problems: Vec<String> = check
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "Line {}, column {}: {}",
                    diagnostic.line, diagnostic.column, diagnostic.message
                )
            })
            .collect();
        let compiles = check.compiles();

        ui.add_space(metric::GAP);
        if compiles {
            panel::note(ui, "Compiles against this build's host.");
        } else {
            for problem in &problems {
                panel::problem(ui, problem);
            }
        }
        self.repair_controls(ui, path, body, compiles);
    }

    fn repair_controls(&mut self, ui: &mut egui::Ui, path: &Path, body: &str, compiles: bool) {
        if self
            .assistant
            .repair
            .offer
            .as_ref()
            .is_some_and(|offer| offer.path == path)
        {
            self.offer_panel(ui);
            return;
        }
        if let Some(asking) = self
            .assistant
            .repair
            .asking
            .as_ref()
            .filter(|asking| asking.path == path)
        {
            let waited = asking.since.elapsed().as_secs();
            let model = asking.model.clone();
            let cancel = Arc::clone(&asking.cancel);
            let note = "The first fix of a session also starts the model, which can take a minute.";
            let mut stop = false;
            ui.horizontal(|ui| {
                ui.add_space(metric::GUTTER);
                ui.label(
                    RichText::new(format!("Asking {model} for a fix… {waited}s"))
                        .size(text::LABEL)
                        .color(color::FORGE_BRIGHT),
                );
                stop = button::labelled(ui, "Stop", Intent::Quiet, "Stop waiting for this fix")
                    .clicked();
            });
            if stop {
                cancel.stop();
                self.assistant.repair.asking = None;
            }
            panel::note(ui, note);
            ui.ctx().request_repaint_after(Duration::from_millis(500));
            return;
        }
        if let Some((_, said)) = self
            .assistant
            .repair
            .outcome
            .as_ref()
            .filter(|(at, _)| at == path)
        {
            panel::note(ui, said);
        }
        if !compiles {
            match self.assistant.repairer() {
                Some((model, connector)) => {
                    ui.horizontal(|ui| {
                        ui.add_space(metric::GUTTER);
                        if button::labelled(
                            ui,
                            "Propose a fix",
                            Intent::Primary,
                            "Asks the local model. Nothing is written until you accept it.",
                        )
                        .clicked()
                        {
                            self.ask_for_fix(path, body, model, connector);
                        }
                    });
                }
                None => panel::note(
                    ui,
                    "Once the local assistant is set up and has passed its script test, it can \
                     propose a fix here.",
                ),
            }
        }
        if self
            .assistant
            .repair
            .previous
            .as_ref()
            .is_some_and(|(at, _)| at == path)
        {
            ui.horizontal(|ui| {
                ui.add_space(metric::GUTTER);
                if button::labelled(
                    ui,
                    "Put the previous version back",
                    Intent::Quiet,
                    "Undoes the accepted fix",
                )
                .clicked()
                {
                    self.put_back(path);
                }
            });
        }
    }

    fn ask_for_fix(&mut self, path: &Path, body: &str, model: String, connector: Connector) {
        let shared = Arc::new(Shared::default());
        let (sender, answer) = channel();
        let file = path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let source = body.to_owned();
        let worker = Arc::clone(&shared);
        std::thread::spawn(move || {
            // Starts the model first if it is resting, which is most of the
            // wait on a first request.
            let answer = connector
                .connect(&worker)
                .map_err(Failure::Unavailable)
                .and_then(|mut runner| repair(&mut runner, &file, &source, &worker.cancel));
            let _ = sender.send(answer);
        });
        self.assistant.repair.outcome = None;
        self.assistant.repair.asking = Some(Asking {
            path: path.to_path_buf(),
            original: body.to_owned(),
            model,
            cancel: shared,
            answer,
            since: Instant::now(),
        });
    }

    /// Takes a finished request's answer, when one has landed.
    fn take_answer(&mut self) {
        let Some(asking) = &self.assistant.repair.asking else {
            return;
        };
        let answer = match asking.answer.try_recv() {
            Ok(answer) => answer,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(Failure::Cancelled),
        };
        let Some(asking) = self.assistant.repair.asking.take() else {
            return;
        };
        match answer {
            Ok(proposal) => {
                self.console.info(format!(
                    "{} proposed a fix for {} that compiles",
                    asking.model,
                    asking.path.display()
                ));
                self.assistant.repair.offer = Some(Offer {
                    diff: diff::lines(&asking.original, &proposal.source),
                    path: asking.path,
                    original: asking.original,
                    proposal,
                });
            }
            Err(failure) => {
                let said = format!(
                    "No fix to offer: {}.",
                    failure.to_string().trim_end_matches('.')
                );
                self.console
                    .info(format!("{}: {said}", asking.path.display()));
                self.assistant.repair.outcome = Some((asking.path, said));
            }
        }
    }

    fn offer_panel(&mut self, ui: &mut egui::Ui) {
        let Some(offer) = &self.assistant.repair.offer else {
            return;
        };
        let (removed, added) = diff::counts(&offer.diff);
        ui.horizontal(|ui| {
            ui.add_space(metric::GUTTER);
            panel::status_dot(ui, color::SUCCESS);
            ui.label(
                RichText::new(format!(
                    "Proposed fix · compiles · {removed} line(s) out, {added} in · \
                     attempt {} of {}",
                    offer.proposal.attempts,
                    1 + MAX_REPAIRS
                ))
                .size(text::LABEL)
                .color(color::TEXT),
            );
        });
        ui.add_space(4.0);
        for line in diff::around_changes(&offer.diff, CONTEXT) {
            let (mark, shown, tint) = match line {
                None => ("", "…", color::TEXT_FAINT),
                Some(Line::Same(line)) => (" ", line.as_str(), color::TEXT_FAINT),
                Some(Line::Removed(line)) => ("-", line.as_str(), color::DANGER_TEXT),
                Some(Line::Added(line)) => ("+", line.as_str(), color::SUCCESS),
            };
            ui.horizontal(|ui| {
                ui.add_space(metric::GUTTER);
                ui.label(
                    RichText::new(format!("{mark} {shown}"))
                        .font(egui::FontId::monospace(text::NOTE))
                        .color(tint),
                );
            });
        }
        ui.add_space(metric::GAP);
        let mut accept = false;
        let mut discard = false;
        ui.horizontal(|ui| {
            ui.add_space(metric::GUTTER);
            accept = button::labelled(ui, "Accept", Intent::Primary, "Write this fix to the file")
                .clicked();
            discard =
                button::labelled(ui, "Discard", Intent::Quiet, "Leave the file as it is").clicked();
        });
        if accept {
            self.accept_offer();
        } else if discard {
            self.assistant.repair.offer = None;
        }
    }

    fn accept_offer(&mut self) {
        let Some(offer) = self.assistant.repair.offer.take() else {
            return;
        };
        // A file changed since the fix was asked for is not the file the fix
        // was made for, and writing over it would lose whatever changed it.
        if std::fs::read_to_string(&offer.path).ok().as_deref() != Some(offer.original.as_str()) {
            self.report(format!(
                "{} changed on disk after the fix was proposed; nothing was written",
                offer.path.display()
            ));
            return;
        }
        match write_whole(&offer.path, &offer.proposal.source) {
            Ok(()) => {
                self.console.info(format!(
                    "Wrote the assistant's fix to {}",
                    offer.path.display()
                ));
                self.assistant.repair.previous = Some((offer.path.clone(), offer.original));
                self.preview = Some(TextPreview::open(&offer.path));
            }
            Err(error) => self.report(format!("Could not write {}: {error}", offer.path.display())),
        }
    }

    fn put_back(&mut self, path: &Path) {
        let Some((_, previous)) = self.assistant.repair.previous.take() else {
            return;
        };
        match write_whole(path, &previous) {
            Ok(()) => {
                self.console
                    .info(format!("Put back the previous {}", path.display()));
                self.preview = Some(TextPreview::open(path));
            }
            Err(error) => self.report(format!("Could not write {}: {error}", path.display())),
        }
    }
}

/// Replaces a file's contents in one step, so an interrupted write leaves the
/// old file rather than half of the new one.
fn write_whole(path: &Path, contents: &str) -> std::io::Result<()> {
    let mut staging = path.as_os_str().to_owned();
    staging.push(".sindri-fix");
    let staging = PathBuf::from(staging);
    std::fs::write(&staging, contents)?;
    std::fs::rename(&staging, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&staging);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_whole_write_replaces_the_file_and_leaves_nothing_beside_it() {
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("hud.decay");
        std::fs::write(&path, "old").expect("seeded");
        write_whole(&path, "new").expect("written");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "new");
        let left: Vec<_> = std::fs::read_dir(directory.path())
            .expect("listed")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(left, [std::ffi::OsString::from("hud.decay")]);
    }
}
