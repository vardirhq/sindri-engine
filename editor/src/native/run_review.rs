//! Editing while a scene plays, and choosing at Stop what to keep.
//!
//! [`crate::run_edits`] holds the rules; this is where the editor's edits go
//! through them and where the choice is offered.

use eframe::egui::{self, Align, Layout, RichText};
use sindri_core::{CommandError, Transaction};

use crate::run_edits::{Verdict, review};
use crate::ui::theme::{color, text};
use crate::ui::widgets::button::{self, Intent};
use crate::ui::widgets::dialog;

use super::EditorApp;

/// The edits a run collected, offered back at Stop.
pub(super) struct StopReview {
    rows: Vec<Row>,
}

struct Row {
    label: String,
    verdict: Verdict,
    keep: bool,
}

impl EditorApp {
    /// Makes an edit: to the scene and its history while stopped, and to the
    /// running world while a scene plays, recorded against the run for Stop
    /// to offer back. Every edit the editor makes comes through here.
    ///
    /// # Errors
    /// The command layer's refusal, with the world unchanged.
    pub(super) fn apply_edit(&mut self, transaction: Transaction) -> Result<(), CommandError> {
        if self.session.is_some() {
            // Made where the run stands, and kept with the recording, so a
            // scrub back past it and forward again finds it there.
            let step = self
                .recording
                .as_ref()
                .map_or(0, crate::recording::Recording::at);
            let recorded = self.recording.is_some().then(|| transaction.clone());
            self.run_edits
                .apply_at(transaction, &mut self.world, step)?;
            if let (Some(recording), Some(edit)) = (self.recording.as_mut(), recorded)
                && let Some(carried_on) = recording.edited(edit)
            {
                self.run_edits.forget_after(carried_on);
            }
            Ok(())
        } else {
            self.history.apply(transaction, &mut self.world)
        }
    }

    /// Ends a continuous interaction, so the next edit is its own step,
    /// whether it lands in the scene's history or the run's.
    pub(super) fn break_merge_runs(&mut self) {
        self.history.break_merge_run();
        self.run_edits.break_merge_run();
    }

    /// Called once Stop has put the scene back: what was edited while it
    /// played is offered, each edit as it would apply to the restored scene.
    pub(super) fn offer_run_edits(&mut self) {
        let edits = self.run_edits.take();
        if edits.is_empty() {
            return;
        }
        let verdicts = review(&edits, &self.world);
        let rows = edits
            .iter()
            .zip(verdicts)
            .map(|(edit, verdict)| Row {
                label: edit.label().to_owned(),
                keep: matches!(verdict, Verdict::Keep(_)),
                verdict,
            })
            .collect();
        self.stop_review = Some(StopReview { rows });
    }

    /// The choice, while there is one to make. Answers whether it is open,
    /// so nothing behind it reads the keyboard meanwhile.
    pub(super) fn stop_review_window(&mut self, context: &egui::Context) -> bool {
        let Some(mut review) = self.stop_review.take() else {
            return false;
        };
        let mut done = None;
        dialog::form(
            context,
            "sindri-run-edits",
            "Changes made while playing",
            |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "Stop put the scene back as it was when Play was pressed. \
                         Keep what you changed while it played?",
                        )
                        .size(text::BODY)
                        .color(color::TEXT_MUTED),
                    )
                    .wrap(),
                );
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        for row in &mut review.rows {
                            match &row.verdict {
                                Verdict::Keep(_) => {
                                    ui.checkbox(&mut row.keep, &row.label);
                                }
                                Verdict::Explained(why) => {
                                    ui.add_enabled(
                                        false,
                                        egui::Checkbox::new(&mut false, &row.label),
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(why)
                                                .size(text::NOTE)
                                                .color(color::TEXT_FAINT),
                                        )
                                        .wrap(),
                                    );
                                }
                            }
                        }
                    });
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if button::labelled(
                        ui,
                        "Discard all",
                        Intent::Quiet,
                        "Keep the scene as it was",
                    )
                    .clicked()
                    {
                        done = Some(false);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if button::labelled(
                            ui,
                            "Keep selected",
                            Intent::Primary,
                            "Apply the ticked changes to the scene, each one undoable",
                        )
                        .clicked()
                        {
                            done = Some(true);
                        }
                    });
                });
            },
        );
        match done {
            Some(true) => self.keep_run_edits(review),
            Some(false) => {}
            None => self.stop_review = Some(review),
        }
        true
    }

    /// Applies the kept edits to the scene as ordinary history entries, in
    /// the order they were made, and says which would not go.
    fn keep_run_edits(&mut self, review: StopReview) {
        for row in review.rows {
            let Verdict::Keep(transaction) = row.verdict else {
                continue;
            };
            if !row.keep {
                continue;
            }
            self.history.break_merge_run();
            if let Err(error) = self.history.apply(transaction, &mut self.world) {
                self.report(format!("Not kept: {}: {error}", row.label));
            }
        }
        self.history.break_merge_run();
    }
}
