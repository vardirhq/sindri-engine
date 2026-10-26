//! Undoable score edits.
use super::{ComposerApp, ScoreSnapshot, UndoKind};
use crate::lab::{live::Command, midi};

impl ComposerApp {
    fn snapshot(&self, candidate: usize, kind: UndoKind) -> ScoreSnapshot {
        let mut racks = self.candidate_racks.clone();
        racks[self.selected] = self.settings.rack.clone();
        ScoreSnapshot {
            candidate,
            kind,
            scores: self.scores.clone(),
            settings: self.settings.clone(),
            generated: self.generated.clone(),
            racks,
            track_locks: self.track_locks,
            sound_locks: self.sound_locks,
            arrangement_locks: self.arrangement_locks,
        }
    }
    pub(super) fn save_undo(&mut self) {
        self.save_undo_kind(UndoKind::Notes);
    }
    pub(super) fn save_undo_kind(&mut self, kind: UndoKind) {
        if self.undo.len() >= 64 {
            self.undo.remove(0);
        }
        self.undo.push(self.snapshot(self.selected, kind));
        self.redo.clear();
    }
    fn restore(&mut self, snapshot: ScoreSnapshot) {
        match snapshot.kind {
            UndoKind::Notes => {
                self.scores[snapshot.candidate].clone_from(&snapshot.scores[snapshot.candidate]);
                self.settings.bpm = snapshot.settings.bpm;
                self.remember_rack();
                self.selected = snapshot.candidate;
                self.settings.rack = self.candidate_racks[self.selected].clone();
            }
            UndoKind::Sounds => {
                self.remember_rack();
                self.selected = snapshot.candidate;
                self.settings.rack = self.candidate_racks[self.selected].clone();
                for layer in 0..5 {
                    self.settings.rack.channel_mut(layer).preset =
                        snapshot.racks[self.selected].channel(layer).preset;
                }
                self.remember_rack();
            }
            UndoKind::Session => {
                self.scores = snapshot.scores;
                self.settings = snapshot.settings;
                self.generated = snapshot.generated;
                self.candidate_racks = snapshot.racks;
                self.track_locks = snapshot.track_locks;
                self.sound_locks = snapshot.sound_locks;
                self.arrangement_locks = snapshot.arrangement_locks;
                self.selected = snapshot.candidate;
                self.settings.rack = self.candidate_racks[self.selected].clone();
                self.seed_input = self
                    .settings
                    .seed_text
                    .clone()
                    .unwrap_or_else(|| self.settings.seed.to_string());
                self.seed_error = None;
            }
        }
        self.selected_note = None;
        self.refresh_arrangement();
        self.fit_pitch_range();
        self.refresh_edited_audio();
    }
    pub(super) fn undo_edit(&mut self) {
        if let Some(snapshot) = self.undo.pop() {
            self.redo
                .push(self.snapshot(snapshot.candidate, snapshot.kind));
            self.restore(snapshot);
            self.status = "Edit undone.".into();
        }
    }
    pub(super) fn redo_edit(&mut self) {
        if let Some(snapshot) = self.redo.pop() {
            self.undo
                .push(self.snapshot(snapshot.candidate, snapshot.kind));
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
                self.restore(snapshot);
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
    pub(super) fn refresh_edited_audio(&mut self) {
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
}
