//! Seed entry changes the next generation, never the song currently playing.
use super::{ComposerApp, theme};
use crate::lab::seed;
use eframe::egui::{self, RichText};
use egui_material_icons::icons;

impl ComposerApp {
    pub(super) fn generation_controls(&mut self, ui: &mut egui::Ui) {
        use crate::lab::Generation;
        ui.horizontal(|ui| {
            ui.label("Composer");
            egui::ComboBox::from_id_salt("generation-version")
                .selected_text(match self.settings.generation {
                    Generation::Legacy => "Legacy",
                    Generation::Musical => "Musical",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.settings.generation,
                        Generation::Musical,
                        "Musical",
                    );
                    ui.selectable_value(
                        &mut self.settings.generation,
                        Generation::Legacy,
                        "Legacy",
                    );
                });
        });
        ui.small("Applies on Generate. Note locks keep the current composer version.");
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::{Generation, composer};

    #[test]
    fn composer_choice_is_a_draft_until_generate_and_survives_save_and_restore() {
        let mut app = ComposerApp::default();
        let original = app.scores.clone();
        assert_eq!(app.selected_settings().generation, Generation::Musical);
        app.settings.generation = Generation::Legacy;
        assert_eq!(app.selected_settings().generation, Generation::Musical);
        assert_eq!(app.scores, original);
        app.generate();
        assert_eq!(app.selected_settings().generation, Generation::Legacy);
        assert_eq!(app.scores[0], composer::compose(&app.selected_settings()));
        let song = app.song_file();
        app.settings.generation = Generation::Musical;
        app.generate();
        app.load_song(song);
        assert_eq!(app.selected_settings().generation, Generation::Legacy);
        assert_eq!(app.settings.generation, Generation::Legacy);
    }

    #[test]
    fn note_locks_keep_the_version_of_the_harmonic_context() {
        let mut app = ComposerApp::default();
        let bass = app.scores[0][0].clone();
        app.track_locks[0][0] = true;
        app.settings.generation = Generation::Legacy;
        app.generate();
        assert_eq!(app.selected_settings().generation, Generation::Musical);
        assert_eq!(app.scores[0][0], bass);
        app.track_locks[0][0] = false;
        app.generate();
        assert_eq!(app.selected_settings().generation, Generation::Legacy);
    }
}
