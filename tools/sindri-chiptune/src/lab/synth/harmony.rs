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

/// Reconstruct the modern pad's path from bar roots. Both preview and playback
/// use this path, so seeking and looping cannot change a chord's inversion.
/// A passing fifth or octave inside a bass riff does not replace its harmony.
pub(super) fn voiced_at(settings: &crate::lab::Settings, bass: &[Note], step: usize) -> Vec<i32> {
    let mut previous = Vec::new();
    for bar in 0..=step.min(crate::lab::STEPS - 1) / 16 {
        if let Some(root) = bass_at(bass, bar * 16, 16) {
            previous = voice_lead(root, settings.key, settings.mood, &previous);
        }
    }
    previous
}

/// Enumerate close-position inversions, minimizing actual individual voice
/// movement rather than just the distance to a chord's average pitch.
fn voice_lead(root: i32, key: i32, mood: Mood, previous: &[i32]) -> Vec<i32> {
    let mut classes: Vec<i32> = chord(root, key, mood, 62.0)
        .iter()
        .map(|pitch| pitch.rem_euclid(12))
        .collect();
    classes.sort_unstable();
    let mut candidates = Vec::new();
    for inversion in 0..classes.len() {
        for octave in 3..=5 {
            let mut pitches = Vec::new();
            for index in 0..classes.len() {
                let mut pitch = classes[(inversion + index) % classes.len()] + octave * 12;
                while pitches.last().is_some_and(|last| pitch <= *last) {
                    pitch += 12;
                }
                pitches.push(pitch);
            }
            if pitches.iter().all(|pitch| (48..=76).contains(pitch)) {
                candidates.push(pitches);
            }
        }
    }
    candidates
        .into_iter()
        .min_by_key(|pitches| {
            let movement: i32 = pitches
                .iter()
                .enumerate()
                .map(|(i, pitch)| previous.get(i).map_or(0, |old| (pitch - old).abs()))
                .sum();
            let center = pitches.iter().sum::<i32>() / i32::try_from(pitches.len()).expect("chord");
            (movement, (center - 62).abs(), pitches.clone())
        })
        .expect("every triad or seventh has a close voicing in this range")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inversions_minimize_voice_motion_and_keep_chord_tones() {
        for mood in [
            Mood::Hopeful,
            Mood::Melancholic,
            Mood::Mysterious,
            Mood::Tense,
        ] {
            for key in 0..12 {
                let mut previous = Vec::new();
                for degree in [0, 4, 5, 3, 1, 4, 0] {
                    let root = 36 + key + scale(mood)[degree];
                    let voiced = voice_lead(root, key, mood, &previous);
                    let mut actual: Vec<_> = voiced.iter().map(|n| n % 12).collect();
                    let mut expected: Vec<_> = chord(root, key, mood, 62.0)
                        .iter()
                        .map(|n| n % 12)
                        .collect();
                    actual.sort_unstable();
                    expected.sort_unstable();
                    assert_eq!(actual, expected);
                    assert!(voiced.windows(2).all(|pair| pair[0] < pair[1]));
                    assert!(voiced.iter().all(|n| (48..=76).contains(n)));
                    // Check every admissible close inversion independently.
                    let cost = |notes: &[i32]| {
                        notes
                            .iter()
                            .zip(&previous)
                            .map(|(a, b)| (a - b).abs())
                            .sum::<i32>()
                    };
                    for bottom in 48..=76 {
                        if !actual.contains(&(bottom % 12)) {
                            continue;
                        }
                        let mut option: Vec<_> = (bottom..bottom + 12)
                            .filter(|n| actual.contains(&(n % 12)))
                            .collect();
                        option.sort_unstable();
                        if option.iter().all(|n| *n <= 76) {
                            assert!(cost(&voiced) <= cost(&option));
                        }
                    }
                    previous = voiced;
                }
            }
        }
    }

    #[test]
    fn bar_harmony_ignores_passing_bass_notes_and_is_seek_independent() {
        let settings = crate::lab::Settings::default();
        let bass = [
            Note {
                pitch: 38,
                start: 0,
                len: 8,
                velocity: 90,
            },
            Note {
                pitch: 33,
                start: 8,
                len: 8,
                velocity: 90,
            },
            Note {
                pitch: 43,
                start: 16,
                len: 16,
                velocity: 90,
            },
        ];
        assert_eq!(
            voiced_at(&settings, &bass, 0),
            voiced_at(&settings, &bass, 8)
        );
        let [_, preview] = crate::lab::synth::generated_layers(&settings, &bass);
        for step in [0, 8, 16, 24] {
            let displayed: Vec<_> = preview
                .iter()
                .filter(|n| n.start <= step && n.start + n.len > step)
                .map(|n| n.pitch)
                .collect();
            assert_eq!(displayed, voiced_at(&settings, &bass, step));
        }
        assert_eq!(
            voiced_at(&settings, &bass, 16),
            voiced_at(&settings, &bass, 24)
        );
    }

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
