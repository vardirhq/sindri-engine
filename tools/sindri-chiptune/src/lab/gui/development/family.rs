use super::super::ComposerApp;
use crate::lab::{
    development,
    song::{MAX_SNAPSHOTS, NamedSnapshot},
};
use eframe::egui;

impl ComposerApp {
    pub(in crate::lab::gui) fn create_soundtrack_family(&mut self) {
        let prefix = self.development.family_name.trim();
        if prefix.is_empty() || prefix.chars().count() > 40 || prefix.chars().any(char::is_control)
        {
            self.status = "Name the family with 1–40 characters on one line.".into();
            return;
        }
        if self.tools.snapshots.len() + 3 > MAX_SNAPSHOTS {
            self.status =
                "A soundtrack family needs room for three snapshots. Remove some snapshots first."
                    .into();
            return;
        }
        let names = development::FAMILY_NAMES.map(|name| format!("{prefix} · {name}"));
        if names
            .iter()
            .any(|name| self.tools.snapshots.iter().any(|take| take.name == *name))
        {
            self.status = "That family name is already used. Choose another name.".into();
            return;
        }
        let family = development::family(&self.song_file());
        // Validate every candidate before storing any of them.
        for song in &family {
            if let Err(error) = crate::lab::song::SongFile::from_json(&song.to_json()) {
                self.status = format!("Could not create family: {error}");
                return;
            }
        }
        self.tools.snapshots.extend(
            names
                .into_iter()
                .zip(family)
                .map(|(name, song)| NamedSnapshot { name, song }),
        );
        self.status = "Created Exploration, Danger and Victory snapshots. Restore one to audition; save the song to keep the family.".into();
    }

    pub(in crate::lab::gui) fn soundtrack_family_controls(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Soundtrack family", |ui| {
            ui.add(egui::TextEdit::singleline(&mut self.development.family_name).char_limit(40).hint_text("Family name").desired_width(ui.available_width()));
            if ui.add_enabled(self.tools.snapshots.len() + 3 <= MAX_SNAPSHOTS, egui::Button::new("Create three related cues")).clicked() { self.create_soundtrack_family(); }
            ui.small("Makes Exploration, Danger and Victory snapshots with related motifs, adapted scales, tempo, dynamics and ensembles. The current song and its locks stay unchanged. Restore versions under Named snapshots, then edit or export each cue.");
        });
    }
}
