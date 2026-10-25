//! Contextual track controls below the score on phones, beside it on desktop.
use super::{ComposerApp, InspectorTab, TRACK_COLORS, overview::LAYERS, theme};
use crate::lab::instruments::Preset;
use eframe::egui::{self, RichText, Vec2};
use egui_material_icons::icons;

impl ComposerApp {
    pub(super) fn inspector(&mut self, ui: &mut egui::Ui) {
        if self.compact {
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 5.0), egui::Sense::hover());
            ui.painter().rect_filled(
                egui::Rect::from_center_size(rect.center(), Vec2::new(30.0, 3.0)),
                2.0,
                theme::FAINT,
            );
        }
        let index = self.selected_layer;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(LAYERS[index])
                    .size(23.0)
                    .strong()
                    .color(TRACK_COLORS[index]),
            );
            if index >= 3 {
                ui.label(
                    RichText::new(if index == 3 {
                        "Generated rhythm"
                    } else {
                        "Follows Bass"
                    })
                    .size(11.0)
                    .color(theme::MUTED),
                );
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let width = (ui.available_width() - 76.0) / 3.0;
            ui.spacing_mut().item_spacing.x = 4.0;
            for (tab, label) in [
                (InspectorTab::Song, "Song"),
                (InspectorTab::Track, "Track"),
                (InspectorTab::Mixer, "Mixer"),
            ] {
                if ui
                    .add_sized(
                        Vec2::new(width, 30.0),
                        egui::Button::new(label).selected(self.inspector_tab == tab),
                    )
                    .clicked()
                {
                    self.inspector_tab = tab;
                }
            }
            ui.menu_button(theme::label(icons::ICON_FOLDER_OPEN, "File"), |ui| {
                ui.set_min_width(260.0);
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| self.song_delivery(ui));
            })
            .response
            .on_hover_text("File / Export");
        });
        ui.add_space(8.0);
        match self.inspector_tab {
            InspectorTab::Song => self.controls(ui),
            InspectorTab::Track => {
                self.selected_track(ui);
                self.ensemble_controls(ui);
            }
            InspectorTab::Mixer => self.instrument_rack(ui),
        }
    }

    fn selected_track(&mut self, ui: &mut egui::Ui) {
        let index = self.selected_layer;
        let color = TRACK_COLORS[index];
        let track = if index == 4 {
            &mut self.settings.rack.pad
        } else {
            &mut self.settings.rack.tracks[index]
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().slider_width = (ui.available_width() - 160.0).clamp(80.0, 180.0);
            ui.label(RichText::new("Level").color(theme::MUTED));
            ui.add(
                egui::Slider::new(&mut track.volume, 0.0..=1.0)
                    .max_decimals(2)
                    .show_value(false),
            );
            ui.label(if track.volume > 0.0 {
                format!("{:.1} dB", 20.0 * track.volume.log10())
            } else {
                "−∞ dB".into()
            });
            theme::toggle(ui, &mut track.muted, "M", theme::DANGER);
            theme::toggle(ui, &mut track.solo, "S", color);
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Preset").color(theme::MUTED));
            egui::ComboBox::from_id_salt(("selected-preset", index))
                .selected_text(track.preset.name())
                .width(ui.available_width().max(90.0))
                .show_ui(ui, |ui| {
                    let presets: &[Preset] = if index == 3 {
                        &Preset::DRUMS
                    } else {
                        &Preset::TONAL
                    };
                    for &preset in presets {
                        ui.selectable_value(&mut track.preset, preset, preset.name());
                    }
                });
        });
        if index < 3 {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Octave").color(theme::MUTED));
                ui.add(
                    egui::DragValue::new(&mut track.octave)
                        .range(-2..=2)
                        .speed(0.1),
                );
                ui.checkbox(&mut self.track_locks[self.selected][index], "Keep notes");
            });
            let locked = self.track_locks[self.selected][index];
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 6.0) / if index == 2 { 2.0 } else { 1.0 };
                if ui
                    .add_enabled(
                        !locked,
                        theme::primary(theme::label(icons::ICON_REFRESH, "Regenerate"))
                            .min_size(Vec2::new(width, 36.0)),
                    )
                    .on_hover_text("Regenerate this track (R)")
                    .clicked()
                {
                    self.regenerate_track();
                }
                if index == 2
                    && ui
                        .add_enabled(
                            !locked,
                            egui::Button::new("Vary melody").min_size(Vec2::new(width, 36.0)),
                        )
                        .on_hover_text("Keep rhythm and ending (V)")
                        .clicked()
                {
                    self.vary_melody();
                }
            });
        } else {
            ui.label(
                RichText::new(if index == 3 {
                    "Rhythm follows the song's mood, density and seed. Change these under Song."
                } else {
                    "Chord voicings follow Bass. Edit Bass to change the harmony."
                })
                .size(12.0)
                .color(theme::MUTED),
            );
        }
    }
}
