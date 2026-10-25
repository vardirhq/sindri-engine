//! Arrangement previews use the exact patterns and voicings used by playback.
use super::{drums, harmony};
use crate::lab::{Note, STEPS, Settings, composer};

pub(in crate::lab) fn generated_layers(settings: &Settings, bass: &[Note]) -> [Vec<Note>; 2] {
    let beat = drums::beat(composer::groove(settings), settings.seed);
    let mut kit = Vec::new();
    let mut chords: Vec<Note> = Vec::new();
    let mut centre = 62.0;
    for step in 0..STEPS {
        for hit in drums::pattern(&beat, settings.energy, step / 16, step % 16) {
            let pitch = match hit.piece {
                drums::Piece::Kick => 36,
                drums::Piece::Snare | drums::Piece::Ghost => 38,
                drums::Piece::TomLow => 41,
                drums::Piece::Closed => 42,
                drums::Piece::TomHigh => 45,
                drums::Piece::Open => 46,
                drums::Piece::Crash => 49,
            };
            // The engine's normalized velocity is finite and bounded by 0..1.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let velocity = (hit.velocity.clamp(0.0, 1.0) * 127.0).round() as u8;
            kit.push(Note {
                pitch,
                start: step,
                len: 1,
                velocity,
            });
        }
        if step % 8 != 0 {
            continue;
        }
        if let Some(root) = harmony::bass_at(bass, step, 8) {
            let voiced = harmony::chord(root, settings.key, settings.mood, centre);
            centre = f64::from(voiced.iter().sum::<i32>())
                / f64::from(u16::try_from(voiced.len()).expect("chord tones"));
            centre = centre.clamp(57.0, 67.0);
            let level = [0.45, 0.55, 0.62, 0.5][step / 128] * (0.7 + 0.3 * settings.energy);
            // The same bounded pad velocity as Engine::change_chord, displayed on a MIDI scale.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let velocity = (level * 127.0).round() as u8;
            chords.extend(voiced.into_iter().map(|pitch| Note {
                pitch,
                start: step,
                len: 8,
                velocity,
            }));
        } else if let Some(last_start) = chords.last().map(|note| note.start) {
            // Playback holds its last chord when there is no new bass root.
            for note in chords
                .iter_mut()
                .rev()
                .take_while(|note| note.start == last_start)
            {
                note.len += 8;
            }
        }
    }
    [kit, chords]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_holds_the_last_chord_through_bass_rests() {
        let bass = [Note {
            pitch: 38,
            start: 0,
            len: 8,
            velocity: 80,
        }];
        let [_, chords] = generated_layers(&Settings::default(), &bass);
        assert!(!chords.is_empty());
        assert!(
            chords
                .iter()
                .all(|note| note.start == 0 && note.len == STEPS)
        );
    }
}
