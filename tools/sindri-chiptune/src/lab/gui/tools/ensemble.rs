use super::super::ComposerApp;
use eframe::egui;

impl ComposerApp {
    pub(in crate::lab::gui) fn try_ensemble(&mut self) {
        let palettes = crate::lab::ensemble::palettes(self.generated.mood);
        let current = &self.settings.rack;
        let next = (1..=3)
            .map(|offset| (self.tools.palette[self.selected] + offset) % 3)
            .find(|index| {
                (0..5).any(|layer| {
                    !self.sound_locks[self.selected][layer]
                        && current.channel(layer).preset != palettes[*index][layer]
                })
            });
        let Some(next) = next else {
            self.status = "Unlock a sound to audition another ensemble.".into();
            return;
        };
        self.save_undo_kind(super::super::UndoKind::Sounds);
        self.tools.palette[self.selected] = next;
        for (layer, &preset) in palettes[next].iter().enumerate() {
            if !self.sound_locks[self.selected][layer] {
                self.settings.rack.channel_mut(layer).preset = preset;
            }
        }
        self.remember_rack();
        self.refresh_edited_audio();
        self.status = format!(
            "Ensemble {} of 3. Notes, levels, tuning and sections were kept. Undo to compare.",
            next + 1
        );
    }

    pub(in crate::lab::gui) fn ensemble_audition_controls(&mut self, ui: &mut egui::Ui) {
        if ui
            .button("Try another ensemble")
            .on_hover_text(
                "Change unlocked sounds, preserving every note and section. Undo to compare.",
            )
            .clicked()
        {
            self.try_ensemble();
        }
    }
}
