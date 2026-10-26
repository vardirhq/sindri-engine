use super::super::{ComposerApp, theme};
use eframe::egui;

impl ComposerApp {
    pub(in crate::lab::gui) fn regenerate_phrase(&mut self) {
        if self.selected_layer >= 3 || self.track_locks[self.selected][self.edit_track] {
            self.status = "Select an unlocked Bass, Arpeggio or Lead layer.".into();
            return;
        }
        if self.tools.first_bar == 0 {
            self.status = "Bar numbers start at 1.".into();
            return;
        }
        let variation = self.regeneration.wrapping_add(1);
        let result = crate::lab::phrases::regenerate(
            &self.selected_settings(),
            &self.scores[self.selected],
            self.edit_track,
            self.tools.first_bar.saturating_sub(1)..self.tools.last_bar,
            variation,
        );
        match result {
            Ok(notes) => {
                self.save_undo();
                self.regeneration = variation;
                self.scores[self.selected][self.edit_track] = notes;
                self.selected_note = None;
                self.section = (self.tools.first_bar - 1) / 8;
                self.focus_score_window();
                self.refresh_edited_audio();
                self.status = format!(
                    "Regenerated bars {}–{}. Surrounding notes were kept.",
                    self.tools.first_bar, self.tools.last_bar
                );
            }
            Err(error) => self.status = error.into(),
        }
    }

    pub(in crate::lab::gui) fn phrase_controls(&mut self, ui: &mut egui::Ui) {
        if self.selected_layer >= 3 {
            return;
        }
        self.tools.range_visible = ui.collapsing("Regenerate selected bars", |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.small_button("Visible bars").clicked() {
                    self.tools.first_bar = self.section * 8 + if self.compact { self.roll_page * 4 } else { 0 } + 1;
                    self.tools.last_bar = self.tools.first_bar + if self.compact { 3 } else { 7 };
                }
                ui.label("Bars");
                ui.add(egui::DragValue::new(&mut self.tools.first_bar).range(1..=32));
                ui.label("through");
                ui.add(egui::DragValue::new(&mut self.tools.last_bar).range(self.tools.first_bar..=32));
            });
            let enabled = self.tools.first_bar <= self.tools.last_bar
                && !self.track_locks[self.selected][self.edit_track];
            if ui.add_enabled(enabled, theme::primary("Regenerate bars")).clicked() {
                self.regenerate_phrase();
            }
            ui.small("Only this note layer changes; Chords still follow Bass. Crossing notes keep their portions outside the range. Undo restores the phrase.");
        }).body_returned.is_some();
    }

    pub(in crate::lab::gui) fn paint_phrase_range(
        &self,
        painter: &egui::Painter,
        grid: egui::Rect,
        begin: usize,
        steps: usize,
    ) {
        if !self.tools.range_visible || self.selected_layer >= 3 {
            return;
        }
        let start = self.tools.first_bar.saturating_sub(1) * 16;
        let end = self.tools.last_bar * 16;
        if end <= begin || start >= begin + steps {
            return;
        }
        let coordinate = |value: usize| f32::from(u16::try_from(value).expect("bounded score"));
        let x = |value: usize| {
            grid.left()
                + grid.width() * coordinate(value.saturating_sub(begin).min(steps))
                    / coordinate(steps)
        };
        let band = egui::Rect::from_min_max(
            egui::pos2(x(start), grid.top()),
            egui::pos2(x(end), grid.top() + 4.0),
        );
        painter.rect_filled(band, 0.0, theme::FORGE);
    }
}
