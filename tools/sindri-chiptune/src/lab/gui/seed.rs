//! Seed entry changes the next generation, never the song currently playing.
use super::{ComposerApp, theme};
use crate::lab::seed;
use eframe::egui::{self, RichText};
use egui_material_icons::icons;

impl ComposerApp {
    pub(super) fn edit_seed(&mut self) {
        match seed::parse(&self.seed_input) {
            Ok(seed) => {
                self.settings.seed = seed.value;
                self.settings.seed_text = seed.text;
                self.seed_error = None;
            }
            Err(error) => self.seed_error = Some(error),
        }
    }

    pub(super) fn randomize_seed(&mut self) {
        match seed::random_different(self.settings.seed) {
            Ok(value) => {
                self.seed_input = value.to_string();
                self.edit_seed();
                self.status = "New random seed selected. Generate to use it.".into();
            }
            Err(error) => self.status = error,
        }
    }

    pub(super) fn seed_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::TextEdit::singleline(&mut self.seed_input)
                        .desired_width(130.0)
                        .hint_text("Number or phrase"),
                )
                .changed()
            {
                self.edit_seed();
            }
            if ui
                .small_button(theme::label(icons::ICON_CASINO, ""))
                .on_hover_text("Choose a completely random seed")
                .clicked()
            {
                self.randomize_seed();
            }
        });
        if let Some(error) = &self.seed_error {
            ui.label(RichText::new(error).size(11.0).color(theme::DANGER));
        } else if self.settings.seed_text.is_some() {
            ui.label(
                RichText::new(format!("Seed {}", self.settings.seed))
                    .size(10.0)
                    .color(theme::MUTED),
            );
        }
    }
}
