use super::super::ComposerApp;
use crate::lab::song::{MAX_SNAPSHOTS, NamedSnapshot};
use eframe::egui;

impl ComposerApp {
    pub(in crate::lab::gui) fn save_named_snapshot(&mut self) {
        let name = self.tools.snapshot_name.trim();
        if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
            self.status = "Name the snapshot with 1–64 characters on one line.".into();
            return;
        }
        if self.tools.snapshots.len() >= MAX_SNAPSHOTS {
            self.status =
                "This song already has 16 snapshots. Remove one before saving another.".into();
            return;
        }
        if self.tools.snapshots.iter().any(|take| take.name == name) {
            self.status = "That snapshot name is already used. Choose a different name.".into();
            return;
        }
        let mut song = self.song_file();
        song.snapshots.clear();
        self.tools.snapshots.push(NamedSnapshot {
            name: name.to_owned(),
            song,
        });
        self.tools.snapshot_name = format!("Version {}", self.tools.snapshots.len() + 1);
        self.status = "Snapshot kept. Save the song to preserve it with the project.".into();
    }

    pub(in crate::lab::gui) fn restore_named_snapshot(&mut self, index: usize) {
        let Some(take) = self.tools.snapshots.get(index).cloned() else {
            return;
        };
        self.save_undo_kind(super::super::UndoKind::Session);
        self.apply_song(take.song);
        self.status = format!(
            "Restored {} into A. Undo returns to your previous candidates.",
            take.name
        );
    }

    pub(in crate::lab::gui) fn snapshot_controls(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Named snapshots", |ui| {
            ui.add(egui::TextEdit::singleline(&mut self.tools.snapshot_name).hint_text("Version name").char_limit(64).desired_width(ui.available_width()));
            if ui.add_enabled(self.tools.snapshots.len() < MAX_SNAPSHOTS, egui::Button::new("Keep snapshot")).clicked() {
                self.save_named_snapshot();
            }
            let mut restore = None;
            let mut remove = None;
            for (index, take) in self.tools.snapshots.iter().enumerate() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(&take.name);
                    if ui.button("Restore").clicked() { restore = Some(index); }
                    if ui.small_button("Remove").clicked() { remove = Some(index); }
                });
            }
            if let Some(index) = restore { self.restore_named_snapshot(index); }
            if let Some(index) = remove { self.tools.snapshots.remove(index); }
            ui.small("Keeps the selected candidate's notes, sounds, sections and song settings. Snapshots travel with the saved song. Restore is undoable.");
        });
    }
}
