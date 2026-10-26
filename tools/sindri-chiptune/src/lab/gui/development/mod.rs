//! Musical development actions sit alongside the existing phrase and snapshot tools.
mod family;
mod phrases;
#[cfg(test)]
mod tests;

use super::ComposerApp;
use crate::lab::{
    Note,
    development::{Dialogue, Feel, Motif},
    midi,
};
use eframe::egui;
use std::ops::Range;

pub(super) struct State {
    motif: Motif,
    dialogue: Dialogue,
    feel: Feel,
    family_name: String,
}
impl Default for State {
    fn default() -> Self {
        Self {
            motif: Motif::default(),
            dialogue: Dialogue::default(),
            feel: Feel::default(),
            family_name: "Theme".into(),
        }
    }
}

impl ComposerApp {
    fn development_bars(&self) -> Result<Range<usize>, &'static str> {
        if self.tools.first_bar == 0 {
            return Err("Bar numbers start at 1.");
        }
        let bars = self.tools.first_bar - 1..self.tools.last_bar;
        crate::lab::development::step_range(bars.clone())?;
        Ok(bars)
    }

    fn development_range(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.small_button("Visible bars").clicked() {
                self.tools.first_bar =
                    self.section * 8 + if self.compact { self.roll_page * 4 } else { 0 } + 1;
                self.tools.last_bar = self.tools.first_bar + if self.compact { 3 } else { 7 };
            }
            ui.label("Source bars");
            ui.add(egui::DragValue::new(&mut self.tools.first_bar).range(1..=32));
            ui.label("through");
            ui.add(egui::DragValue::new(&mut self.tools.last_bar).range(self.tools.first_bar..=32));
        });
    }

    fn commit_development(
        &mut self,
        result: Result<[Vec<Note>; 3], &'static str>,
        message: &str,
        focus_bar: usize,
    ) {
        match result.and_then(|tracks| {
            midi::validate(&tracks)?;
            Ok(tracks)
        }) {
            Ok(tracks) => {
                if tracks == self.scores[self.selected] {
                    self.status = "These settings leave the score unchanged.".into();
                    return;
                }
                self.save_undo();
                self.scores[self.selected] = tracks;
                self.regeneration = self.regeneration.wrapping_add(1);
                self.selected_note = None;
                self.section = focus_bar / 8;
                self.roll_page = (focus_bar % 8) / 4;
                self.focus_score_window();
                self.refresh_edited_audio();
                self.status = format!("{message} Undo compares with the previous version.");
            }
            Err(error) => self.status = error.into(),
        }
    }

    pub(super) fn musical_development_controls(&mut self, ui: &mut egui::Ui) {
        if self.selected_layer >= 3 {
            return;
        }
        let motif = ui.collapsing("Develop a motif", |ui| self.motif_controls(ui));
        let dialogue = ui.collapsing("Call and response", |ui| self.dialogue_controls(ui));
        let feel = ui.collapsing("Performance expression", |ui| self.feel_controls(ui));
        self.tools.range_visible |= motif.body_returned.is_some()
            || dialogue.body_returned.is_some()
            || feel.body_returned.is_some();
    }
}
