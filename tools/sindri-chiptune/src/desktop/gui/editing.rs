//! Undoable score edits and MIDI file actions.
use super::ComposerApp;
use crate::desktop::{live::Command, midi};
use eframe::egui;

impl ComposerApp {
    pub(super) fn save_undo(&mut self) {
        if self.undo.len() >= 64 {
            self.undo.remove(0);
        }
        self.undo
            .push((self.selected, self.scores[self.selected].clone()));
        self.redo.clear();
    }
    pub(super) fn undo_edit(&mut self) {
        if let Some((candidate, score)) = self.undo.pop() {
            self.redo.push((candidate, self.scores[candidate].clone()));
            self.scores[candidate] = score;
            self.selected = candidate;
            self.selected_note = None;
            self.refresh_edited_audio();
        }
    }
    pub(super) fn redo_edit(&mut self) {
        if let Some((candidate, score)) = self.redo.pop() {
            self.undo.push((candidate, self.scores[candidate].clone()));
            self.scores[candidate] = score;
            self.selected = candidate;
            self.selected_note = None;
            self.refresh_edited_audio();
        }
    }
    pub(super) fn finish_edit(&mut self) {
        let selected = self.selected_note.and_then(|index| {
            self.scores[self.selected][self.edit_track]
                .get(index)
                .copied()
        });
        for track in &mut self.scores[self.selected] {
            track.sort_by_key(|n| n.start);
        }
        // The synth has one voice per part: reject overlaps instead of hiding notes.
        if let Err(error) = midi::validate(&self.scores[self.selected]) {
            if let Some((candidate, score)) = self.undo.pop() {
                self.scores[candidate] = score;
            }
            self.selected_note = None;
            self.status = error.into();
            return;
        }
        self.selected_note = selected.and_then(|note| {
            self.scores[self.selected][self.edit_track]
                .iter()
                .position(|n| *n == note)
        });
        self.refresh_edited_audio();
        self.status = "Score edited. WAV and MIDI export use these notes.".into();
    }
    fn refresh_edited_audio(&mut self) {
        if let Some(transport) = &self.transport {
            let _ = transport
                .tx
                .send(Command::Edit(self.scores[self.selected].clone()));
        }
    }
    pub(super) fn midi_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("MIDI path");
            ui.add(egui::TextEdit::singleline(&mut self.midi_path).desired_width(200.0));
            if ui.add_enabled(!self.midi_path.trim().is_empty(), egui::Button::new("Export MIDI")).clicked() {
                self.status = match midi::write(&self.midi_path, &self.selected_settings(), &self.scores[self.selected]) {
                    Ok(()) => format!("Saved {}. MIDI carries the three tonal parts; chip timbres stay here.", self.midi_path),
                    Err(error) => format!("MIDI export failed: {error}"),
                };
            }
            if ui.add_enabled(!self.midi_path.trim().is_empty(), egui::Button::new("Import MIDI")).clicked() {
                match std::fs::read(&self.midi_path).map_err(|e| e.to_string()).and_then(|data| midi::read(&data)) {
                    Ok((notes, bpm)) => {
                        self.save_undo(); self.scores[self.selected] = notes;
                        self.settings.bpm = bpm; self.selected_note = None;
                        self.refresh_edited_audio();
                        self.status = "Imported MIDI, snapped to 1/16. Three monophonic tonal parts, up to 32 bars.".into();
                    }
                    Err(error) => self.status = format!("MIDI import failed: {error}"),
                }
            }
            ui.small("MIDI: note data / WAV: finished sound");
        });
    }
}
