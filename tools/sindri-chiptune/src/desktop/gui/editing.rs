//! Undoable score edits and MIDI file actions.
use super::{ComposerApp, ScoreSnapshot};
use crate::desktop::{live::Command, midi};
use eframe::egui;

impl ComposerApp {
    fn snapshot(&self, candidate: usize) -> ScoreSnapshot {
        ScoreSnapshot {
            candidate,
            notes: self.scores[candidate].clone(),
            bpm: self.settings.bpm,
        }
    }
    pub(super) fn save_undo(&mut self) {
        if self.undo.len() >= 64 {
            self.undo.remove(0);
        }
        self.undo.push(self.snapshot(self.selected));
        self.redo.clear();
    }
    fn restore(&mut self, snapshot: ScoreSnapshot) {
        self.scores[snapshot.candidate] = snapshot.notes;
        self.settings.bpm = snapshot.bpm;
        self.selected = snapshot.candidate;
        self.selected_note = None;
        self.fit_pitch_range();
        self.refresh_edited_audio();
    }
    pub(super) fn undo_edit(&mut self) {
        if let Some(snapshot) = self.undo.pop() {
            self.redo.push(self.snapshot(snapshot.candidate));
            self.restore(snapshot);
            self.status = "Edit undone.".into();
        }
    }
    pub(super) fn redo_edit(&mut self) {
        if let Some(snapshot) = self.redo.pop() {
            self.undo.push(self.snapshot(snapshot.candidate));
            self.restore(snapshot);
            self.status = "Edit restored.".into();
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
            if let Some(snapshot) = self.undo.pop() {
                self.scores[snapshot.candidate] = snapshot.notes;
                self.settings.bpm = snapshot.bpm;
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
            let settings = self.selected_settings();
            let _ = transport.tx.send(Command::Update(settings.clone()));
            let _ = transport
                .tx
                .send(Command::Edit(self.scores[self.selected].clone()));
            self.playing = Some(settings.clone());
            self.last_sent = Some(settings);
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
                        self.settings.bpm = bpm; self.selected_note = None; self.fit_pitch_range();
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
