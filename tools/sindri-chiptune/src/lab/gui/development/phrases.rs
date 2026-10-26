use super::super::ComposerApp;
use crate::lab::development::{self, Accent, Shape};
use eframe::egui;

impl ComposerApp {
    pub(in crate::lab::gui) fn apply_motif(&mut self) {
        if self.selected_layer >= 3 || self.track_locks[self.selected][self.edit_track] {
            self.status = "Select an unlocked melodic layer to develop its motif.".into();
            return;
        }
        let result = self.development_bars().and_then(|bars| {
            development::develop_motif(
                &self.selected_settings(),
                &self.scores[self.selected],
                self.edit_track,
                bars,
                &self.development.motif,
            )
        });
        self.commit_development(
            result,
            "Motif developed in the destination bars.",
            self.development.motif.target,
        );
    }

    pub(in crate::lab::gui) fn apply_dialogue(&mut self) {
        if self.track_locks[self.selected][1] || self.track_locks[self.selected][2] {
            self.status = "Unlock Arpeggio and Lead before arranging a dialogue.".into();
            return;
        }
        let result = self.development_bars().and_then(|bars| {
            development::arrange_dialogue(
                &self.selected_settings(),
                &self.scores[self.selected],
                bars,
                &self.development.dialogue,
            )
        });
        self.commit_development(
            result,
            "Arpeggio and Lead now trade call and response phrases.",
            self.tools.first_bar.saturating_sub(1),
        );
    }

    pub(in crate::lab::gui) fn apply_feel(&mut self) {
        if self.selected_layer >= 3 || self.track_locks[self.selected][self.edit_track] {
            self.status = "Select an unlocked melodic layer to shape its performance.".into();
            return;
        }
        let result = self.development_bars().and_then(|bars| {
            let mut tracks = self.scores[self.selected].clone();
            tracks[self.edit_track] = development::shape_performance(
                &self.selected_settings(),
                &tracks[self.edit_track],
                bars,
                &self.development.feel,
                self.regeneration.wrapping_add(1),
            )?;
            Ok(tracks)
        });
        self.commit_development(
            result,
            "Performance shaped; pitch and onset timing were kept.",
            self.tools.first_bar.saturating_sub(1),
        );
    }

    pub(super) fn motif_controls(&mut self, ui: &mut egui::Ui) {
        self.development_range(ui);
        let options = &mut self.development.motif;
        let mut destination = options.target + 1;
        ui.horizontal_wrapped(|ui| {
            ui.label("Destination bar");
            ui.add(egui::DragValue::new(&mut destination).range(1..=32));
            options.target = destination - 1;
            ui.label("Length factor");
            ui.add(egui::DragValue::new(&mut options.stretch).range(1..=2));
        });
        egui::ComboBox::from_id_salt("motif-shape")
            .selected_text(match options.shape {
                Shape::Repeat => "Repeat",
                Shape::Sequence => "Scale sequence",
                Shape::Invert => "Invert contour",
            })
            .show_ui(ui, |ui| {
                for (value, name) in [
                    (Shape::Repeat, "Repeat"),
                    (Shape::Sequence, "Scale sequence"),
                    (Shape::Invert, "Invert contour"),
                ] {
                    ui.selectable_value(&mut options.shape, value, name);
                }
            });
        if options.shape == Shape::Sequence {
            ui.add(egui::Slider::new(&mut options.steps, -7..=7).text("Scale steps"));
        }
        ui.checkbox(
            &mut options.follow_harmony,
            "Follow destination bass harmony",
        );
        if ui
            .add_enabled(
                !self.track_locks[self.selected][self.edit_track],
                egui::Button::new("Develop motif"),
            )
            .clicked()
        {
            self.apply_motif();
        }
        ui.small("Replaces the destination in this layer. A length factor of 2 stretches the phrase. Bass edits also affect Chords.");
    }

    pub(super) fn dialogue_controls(&mut self, ui: &mut egui::Ui) {
        self.development_range(ui);
        ui.horizontal_wrapped(|ui| {
            ui.label("Caller");
            ui.selectable_value(&mut self.development.dialogue.caller, 2, "Lead");
            ui.selectable_value(&mut self.development.dialogue.caller, 1, "Arpeggio");
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Bars per phrase");
            ui.add(egui::DragValue::new(&mut self.development.dialogue.phrase_bars).range(1..=2));
        });
        let unlocked = !self.track_locks[self.selected][1] && !self.track_locks[self.selected][2];
        if ui
            .add_enabled(unlocked, egui::Button::new("Arrange dialogue"))
            .clicked()
        {
            self.apply_dialogue();
        }
        ui.small("Rewrites both Arpeggio and Lead in these bars into alternating calls and answers. Bass, section switches and surrounding bars stay intact.");
    }

    pub(super) fn feel_controls(&mut self, ui: &mut egui::Ui) {
        self.development_range(ui);
        ui.add(
            egui::Slider::new(&mut self.development.feel.strength, 0.0..=1.0).text("Expression"),
        );
        ui.add(egui::Slider::new(&mut self.development.feel.gate, 50..=100).text("Note length %"));
        egui::ComboBox::from_id_salt("performance-accent")
            .selected_text(match self.development.feel.accent {
                Accent::Downbeats => "Downbeats",
                Accent::Backbeat => "Backbeat",
                Accent::Offbeats => "Offbeats",
            })
            .show_ui(ui, |ui| {
                for (value, name) in [
                    (Accent::Downbeats, "Downbeats"),
                    (Accent::Backbeat, "Backbeat"),
                    (Accent::Offbeats, "Offbeats"),
                ] {
                    ui.selectable_value(&mut self.development.feel.accent, value, name);
                }
            });
        if ui
            .add_enabled(
                !self.track_locks[self.selected][self.edit_track],
                egui::Button::new("Shape performance"),
            )
            .clicked()
        {
            self.apply_feel();
        }
        ui.small("Shapes accents, phrase dynamics and articulation. Pitches and note onsets stay fixed. Notes crossing range boundaries stay untouched.");
    }
}
