use super::super::{ComposerApp, theme};
use crate::lab::{Note, Settings, instruments::Preset, synth};
use eframe::egui::{self, RichText};

const NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
fn class(pitch: i32) -> &'static str {
    NAMES[usize::try_from(pitch.rem_euclid(12)).expect("pitch class")]
}

/// Describes note relationships, rather than labeling dissonance as an error.
fn describe(note: Note, settings: &Settings, chords: &[Note]) -> String {
    let active: Vec<_> = chords
        .iter()
        .filter(|chord| chord.start <= note.start && chord.start + chord.len > note.start)
        .collect();
    let in_scale =
        synth::scale(settings.mood).contains(&(note.pitch - settings.key).rem_euclid(12));
    let chord_tone = active
        .iter()
        .any(|chord| (chord.pitch - note.pitch).rem_euclid(12) == 0);
    let relation = if chord_tone {
        "Chord tone"
    } else if in_scale {
        "Scale tone outside the chord"
    } else {
        "Outside the song's scale"
    };
    let tones = active
        .iter()
        .map(|chord| class(chord.pitch))
        .collect::<Vec<_>>()
        .join(" · ");
    let harmony = if tones.is_empty() {
        "No chord sounding"
    } else {
        &tones
    };
    let crosses = chords
        .iter()
        .any(|chord| chord.start > note.start && chord.start < note.start + note.len);
    format!(
        "{}{} · {relation}\nWritten chord at onset: {harmony}{}",
        class(note.pitch),
        note.pitch.div_euclid(12) - 1,
        if crosses {
            "\nSustains across a chord boundary"
        } else {
            ""
        }
    )
}

impl ComposerApp {
    pub(in crate::lab::gui) fn harmony_controls(&mut self, ui: &mut egui::Ui) {
        if self.selected_layer >= 3 {
            return;
        }
        ui.collapsing("Inspect harmony", |ui| {
            self.refresh_arrangement();
            let note = self.selected_note.and_then(|index| self.scores[self.selected][self.edit_track].get(index)).copied();
            if let Some(mut note) = note {
                let settings = self.selected_settings();
                note.pitch += settings.rack.channel(self.edit_track).octave * 12;
                ui.label(RichText::new(describe(note, &settings, self.layer_notes(4))).color(theme::TEXT));
                if !settings.rack.arrangement.active(note.start / 128, 4) || !settings.rack.audible(4) {
                    ui.small("The Chords layer is currently silenced.");
                }
                let layer = self.edit_track;
                if !settings.rack.arrangement.active(note.start / 128, layer) || !settings.rack.audible(layer) {
                    ui.small("This note is currently silenced by section or mixer settings.");
                }
                if settings.rack.channel(layer).preset == Preset::Detuned {
                    ui.small("Detuned Pulse layers slightly detuned voices; beating is part of its sound.");
                }
                ui.small("A tense interval can be intentional. This inspects written harmony, not the audio signal or effect tails.");
            } else {
                ui.small("Select a piano-roll note to inspect its pitch, scale and chord relationship.");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inspection_distinguishes_chord_scale_and_chromatic_tones_and_boundaries() {
        let settings = Settings {
            mood: crate::lab::Mood::Hopeful,
            ..Settings::default()
        };
        let note = Note {
            pitch: 62,
            start: 0,
            len: 16,
            velocity: 80,
        };
        let chords = [
            Note {
                pitch: 62,
                start: 0,
                len: 8,
                velocity: 80,
            },
            Note {
                pitch: 66,
                start: 8,
                len: 8,
                velocity: 80,
            },
        ];
        assert!(describe(note, &settings, &chords).contains("Chord tone"));
        assert!(describe(note, &settings, &chords).contains("Sustains"));
        assert!(
            describe(Note { pitch: 64, ..note }, &settings, &chords).contains("Scale tone outside")
        );
        assert!(
            describe(Note { pitch: 63, ..note }, &settings, &chords).contains("Outside the song")
        );
        assert!(describe(note, &settings, &[]).contains("No chord sounding"));
    }
}
