//! The chords under a song, read from its bass line so they follow every edit.
use crate::lab::{Mood, Note};

/// Each mood's scale, as semitones above the key.
pub(in crate::lab) fn scale(mood: Mood) -> [i32; 7] {
    match mood {
        Mood::Mysterious => [0, 2, 3, 5, 7, 9, 10],
        Mood::Hopeful => [0, 2, 4, 5, 7, 9, 11],
        Mood::Tense => [0, 1, 3, 5, 7, 8, 10],
        Mood::Melancholic => [0, 2, 3, 5, 7, 8, 10],
    }
}

/// The bass note that defines the harmony from `from` for `length` steps:
/// the one sounding on its downbeat, or failing that the first to begin in it.
/// Not the lowest: a riff that drops to the fifth below is still on the root's
/// chord.
pub(in crate::lab) fn bass_at(bass: &[Note], from: usize, length: usize) -> Option<i32> {
    bass.iter()
        .find(|n| n.start <= from && n.start + n.len > from)
        .or_else(|| {
            bass.iter()
                .filter(|n| n.start > from && n.start < from + length)
                .min_by_key(|n| n.start)
        })
        .map(|n| n.pitch)
}

/// A chord built on `root` by stacking thirds in the song's scale, voiced
/// near `around` so each change moves as little as it can.
pub(super) fn chord(root: i32, key: i32, mood: Mood, around: f64) -> Vec<i32> {
    let scale = scale(mood);
    let class = (root - key).rem_euclid(12);
    let tones: Vec<i32> = if let Some(degree) = scale.iter().position(|s| *s == class) {
        // Sevenths colour the darker moods; the brighter ones stay triadic.
        let size = if matches!(mood, Mood::Mysterious | Mood::Melancholic) {
            4
        } else {
            3
        };
        (0..size)
            .map(|i| {
                let d = degree + i * 2;
                let octave = i32::try_from(d / 7).expect("small");
                key + scale[d % 7] + 12 * octave
            })
            .collect()
    } else {
        // An edited bass note outside the scale gets a bare fifth: never wrong.
        vec![root, root + 7]
    };
    let mut voiced: Vec<i32> = tones
        .iter()
        .map(|tone| {
            let class = tone.rem_euclid(12);
            // The octave of this pitch class nearest the last chord's centre.
            (3..7)
                .map(|octave| class + 12 * octave)
                .min_by(|a, b| {
                    (f64::from(*a) - around)
                        .abs()
                        .total_cmp(&(f64::from(*b) - around).abs())
                })
                .expect("octaves")
        })
        .collect();
    voiced.sort_unstable();
    voiced.dedup();
    voiced
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chords_stay_in_key_and_move_smoothly() {
        let key = 2;
        let first = chord(38, key, Mood::Hopeful, 62.0);
        let second = chord(43, key, Mood::Hopeful, 62.0);
        for note in first.iter().chain(&second) {
            assert!(scale(Mood::Hopeful).contains(&(note - key).rem_euclid(12)));
            assert!((48..=84).contains(note), "{note}");
        }
        let moved: i32 = first.iter().zip(&second).map(|(a, b)| (a - b).abs()).sum();
        assert!(moved <= 8, "voice leading moved {moved} semitones");
    }

    #[test]
    fn darker_moods_get_sevenths_and_strangers_get_fifths() {
        assert_eq!(chord(38, 2, Mood::Melancholic, 62.0).len(), 4);
        assert_eq!(chord(39, 2, Mood::Hopeful, 62.0).len(), 2);
    }

    #[test]
    fn the_harmony_follows_the_bass_on_the_downbeat() {
        let bass = [
            Note {
                pitch: 40,
                start: 0,
                len: 6,
                velocity: 90,
            },
            Note {
                pitch: 36,
                start: 4,
                len: 2,
                velocity: 90,
            },
        ];
        assert_eq!(bass_at(&bass, 0, 8), Some(40));
        assert_eq!(bass_at(&bass, 2, 8), Some(40));
        assert_eq!(bass_at(&bass, 8, 8), None);
    }
}
