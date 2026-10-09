//! Scrubbing a recorded run from the Timeline.
//!
//! [`crate::recording`] keeps the run; this takes it to a step. Scrubbing
//! pauses the run, puts the world and session back to the recorded copy at or
//! before the step, and steps forward with the recorded input, laying the
//! screen out after the steps the Game view was drawn after, as it was then.
//! Edits made during the run are made again before the steps they were made
//! before. Resuming carries on from there and lets the recorded future go,
//! the edits in it included.

use eframe::egui::{self, RichText};
use sindri_core::EngineState;

use crate::recording::{EVERY, KEEP};
use crate::ui::theme::{color, text};

use super::EditorApp;

/// The fixed rate a recording's steps are counted in.
const STEPS_PER_SECOND: f64 = 60.0;

impl EditorApp {
    /// The Timeline's strip for a recorded run: where the run is, and a way
    /// back through it. Nothing while no scene plays.
    pub(super) fn run_scrubber(&mut self, ui: &mut egui::Ui) {
        let Some(recording) = self.recording.as_ref() else {
            return;
        };
        let (from, to) = recording.range();
        let mut at = recording.at();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Run").strong().color(color::TEXT));
            let slider = egui::Slider::new(&mut at, from..=to.max(from))
                .custom_formatter(|step, _| format!("{:.1} s", step / STEPS_PER_SECOND))
                .clamping(egui::SliderClamping::Always);
            if ui
                .add(slider)
                .on_hover_text("Drag back through the run. Playing on carries on from here.")
                .changed()
            {
                self.scrub_to(at);
            }
        });
        #[allow(clippy::cast_precision_loss)]
        let window = (EVERY as f64 * KEEP as f64) / STEPS_PER_SECOND;
        ui.label(
            RichText::new(format!(
                "Recorded: the last {window:.0} seconds, a copy of the run each second"
            ))
            .size(text::NOTE)
            .color(color::TEXT_FAINT),
        );
        ui.separator();
    }

    /// Takes the run to recorded step `target`, pausing it there.
    pub(super) fn scrub_to(&mut self, target: u64) {
        if self.lifecycle.state() == EngineState::Running {
            self.toggle_pause();
        }
        let (Some(recording), Some(session)) = (self.recording.as_mut(), self.session.as_mut())
        else {
            return;
        };
        let Some(replay) = recording.seek(target) else {
            return;
        };
        self.world = replay.mark.world.clone();
        session.restore(&replay.mark.session);
        let components = self.scene.components().clone();
        // Edits are made again as they were made, without a history: they
        // are already recorded against the run.
        let mut remade = sindri_core::CommandHistory::with_limit(0);
        let mut edits = replay.edits.into_iter().peekable();
        for (at, step) in (replay.mark.step..).zip(replay.steps) {
            while let Some((_, edit)) = edits.next_if(|(made, _)| *made == at) {
                if let Err(error) = remade.apply(edit, &mut self.world) {
                    self.console.error(format!("Scrub: {error}"));
                }
            }
            if let Err(error) =
                session.step(&mut self.world, &step.input, step.viewport, step.delta)
            {
                self.console.error(format!("Scrub: {error}"));
                break;
            }
            // Heard the first time through; not again.
            drop(session.take_audio_commands());
            if let Some(view) = step.drawn {
                // As the Game view's draw did: styled in place, the text
                // measured by the renderer that draws it, the layout
                // recorded for the next step's clicks, the styling undone.
                let undo = session.style(&mut self.world, view).ok().flatten();
                let sizes = sindri_scene::measure_ui_text(
                    &self.world,
                    &components,
                    &mut self.renderers.text,
                )
                .unwrap_or_default();
                let _ = session.record_drawn(&self.world, view, sizes);
                if let Some(undo) = undo {
                    undo.undo(&mut self.world);
                }
            }
        }
        for (_, edit) in edits {
            if let Err(error) = remade.apply(edit, &mut self.world) {
                self.console.error(format!("Scrub: {error}"));
            }
        }
        recording.scrubbed_to(target);
        self.break_merge_runs();
    }
}
