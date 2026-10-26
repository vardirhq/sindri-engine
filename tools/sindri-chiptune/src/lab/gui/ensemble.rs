//! Seeded ensembles, per-candidate mixer state, and separate preservation controls.
use super::{ComposerApp, theme};
use crate::lab::{Settings, ensemble, instruments::Rack};
use eframe::egui::{self, RichText};

pub(super) struct Options {
    pub(super) sounds: bool,
    pub(super) arrangement: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            sounds: true,
            arrangement: true,
        }
    }
}

impl ComposerApp {
    pub(super) fn ensemble_candidates(settings: &Settings) -> [Rack; 2] {
        std::array::from_fn(|candidate| {
            let mut settings = settings.clone();
            settings.seed = settings
                .seed
                .wrapping_add(u64::try_from(candidate).expect("candidate"));
            ensemble::generate(&settings)
        })
    }

    pub(super) fn remember_rack(&mut self) {
        self.candidate_racks[self.selected] = self.settings.rack.clone();
    }

    pub(super) fn generate_ensembles(&mut self) {
        self.remember_rack();
        let choices = Self::ensemble_candidates(&self.settings);
        for (candidate, choice) in choices.iter().enumerate() {
            let rack = &mut self.candidate_racks[candidate];
            for layer in 0..5 {
                if self.ensemble_options.sounds && !self.sound_locks[candidate][layer] {
                    // Mixing and tuning are user decisions; only the patch is seeded.
                    rack.channel_mut(layer).preset = choice.channel(layer).preset;
                }
                if self.ensemble_options.arrangement && !self.arrangement_locks[candidate][layer] {
                    for section in 0..4 {
                        rack.arrangement.set_active(
                            section,
                            layer,
                            choice.arrangement.active(section, layer),
                        );
                    }
                }
            }
            if self.ensemble_options.arrangement {
                rack.arrangement.version = ensemble::VERSION;
            }
        }
        self.settings.rack = self.candidate_racks[0].clone();
    }

    pub(super) fn ensemble_controls(&mut self, ui: &mut egui::Ui) {
        let layer = self.selected_layer;
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.sound_locks[self.selected][layer], "Keep sound")
                .on_hover_text("Keep this instrument preset when generating a new ensemble.");
            ui.checkbox(
                &mut self.arrangement_locks[self.selected][layer],
                "Keep arrangement",
            )
            .on_hover_text("Keep this layer's section participation on Generate.");
        });
        ui.label(RichText::new("PLAYS IN").size(10.5).color(theme::MUTED));
        ui.horizontal_wrapped(|ui| {
            for (section, label) in ["Intro", "Theme", "Rise", "Return"].iter().enumerate() {
                let mut active = self.settings.rack.arrangement.active(section, layer);
                if theme::toggle(ui, &mut active, label, super::TRACK_COLORS[layer]) {
                    self.settings
                        .rack
                        .arrangement
                        .set_active(section, layer, active);
                }
            }
        });
        ui.small("Dimmed sections rest. Notes stay available for editing.");
    }
}
