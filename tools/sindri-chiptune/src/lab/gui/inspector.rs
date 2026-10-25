//! One contextual inspector for song, selected-track, and full-mix controls.
use super::{ComposerApp, InspectorTab, TRACK_COLORS, theme};
use crate::lab::instruments::Preset;
use eframe::egui::{self, RichText, Vec2};

impl ComposerApp {
    pub(super) fn inspector(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            for (tab, label) in [
                (InspectorTab::Song, "Song"),
                (InspectorTab::Track, "Track"),
                (InspectorTab::Mixer, "Mixer"),
            ] {
                if theme::segment(ui, self.inspector_tab == tab, label) {
                    self.inspector_tab = tab;
                }
            }
        });
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        match self.inspector_tab {
            InspectorTab::Song => {
                self.controls(ui);
                ui.add_space(8.0);
                self.song_delivery(ui);
            }
            InspectorTab::Track => self.selected_track(ui),
            InspectorTab::Mixer => self.instrument_rack(ui),
        }
    }

    fn selected_track(&mut self, ui: &mut egui::Ui) {
        let index = self.edit_track.min(2);
        let title = ["Bass", "Arpeggio", "Lead"][index];
        let color = TRACK_COLORS[index];
        theme::card(ui, &format!("SELECTED TRACK · {title}"), color, |ui| {
            ui.small("Track settings update the current mix immediately.");
            ui.add_space(8.0);
            let track = &mut self.settings.rack.tracks[index];
            ui.horizontal(|ui| {
                ui.label(RichText::new("Preset").color(theme::MUTED));
                egui::ComboBox::from_id_salt(("selected-preset", index))
                    .selected_text(track.preset.name())
                    .width(ui.available_width().max(90.0))
                    .show_ui(ui, |ui| {
                        for &preset in &Preset::TONAL {
                            ui.selectable_value(&mut track.preset, preset, preset.name());
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Octave").color(theme::MUTED));
                ui.add(
                    egui::DragValue::new(&mut track.octave)
                        .range(-2..=2)
                        .speed(0.1),
                );
            });
            ui.add(
                egui::Slider::new(&mut track.volume, 0.0..=1.0)
                    .show_value(true)
                    .max_decimals(2)
                    .text("Level"),
            );
            ui.horizontal(|ui| {
                theme::toggle(ui, &mut track.muted, "Mute", theme::DANGER);
                theme::toggle(ui, &mut track.solo, "Solo", theme::FORGE_BRIGHT);
            });
            ui.add_space(6.0);
            ui.checkbox(
                &mut self.track_locks[self.selected][index],
                "Keep on Generate",
            )
            .on_hover_text("Preserve this track when generating new candidates.");
            let locked = self.track_locks[self.selected][index];
            ui.add_space(6.0);
            if ui
                .add_enabled(
                    !locked,
                    theme::primary("Regenerate track (R)")
                        .min_size(Vec2::new(ui.available_width(), 32.0)),
                )
                .clicked()
            {
                self.regenerate_track();
            }
            if index == 2
                && ui
                    .add_enabled(
                        !locked,
                        egui::Button::new("Vary melody (V)")
                            .min_size(Vec2::new(ui.available_width(), 30.0)),
                    )
                    .on_hover_text("Create an in-key variation while keeping the rhythm and ending.")
                    .clicked()
            {
                self.vary_melody();
            }
            if index == 2 {
                ui.label(
                    RichText::new("Variation keeps the rhythm and ending.")
                        .size(10.5)
                        .color(theme::FAINT),
                );
            }
        });
    }
}
