use super::*;

fn compose_with(mood: Mood, seed: u64, energy: f64) -> [Vec<Note>; 3] {
    compose(&Settings {
        seed,
        energy,
        mood,
        ..Settings::default()
    })
}

const MOODS: [Mood; 4] = [
    Mood::Mysterious,
    Mood::Hopeful,
    Mood::Tense,
    Mood::Melancholic,
];

#[test]
fn moods_have_distinct_melodic_and_harmonic_patterns() {
    let mut settings = Settings::default();
    let original = compose(&settings);
    settings.mood = Mood::Hopeful;
    let bright = compose(&settings);
    settings.mood = Mood::Tense;
    let tense = compose(&settings);
    assert_ne!(original[0], bright[0]);
    assert_ne!(bright[2], tense[2]);
}

#[test]
fn scores_are_monophonic_in_key_and_resolve_to_tonic() {
    for mood in MOODS {
        for seed in 0..64 {
            let settings = Settings {
                mood,
                seed,
                energy: 0.9,
                ..Settings::default()
            };
            let tracks = compose(&settings);
            crate::lab::midi::validate(&tracks).expect("bounded monophonic score");
            let scale = crate::lab::synth::scale(mood);
            for note in tracks.iter().flatten() {
                assert!(scale.contains(&(note.pitch - settings.key).rem_euclid(12)));
            }
            let ending = tracks[2].last().expect("ending");
            assert_eq!((ending.pitch - settings.key).rem_euclid(12), 0);
            assert!(ending.start + ending.len < crate::lab::STEPS);
        }
    }
}

#[test]
fn the_theme_comes_back_note_for_note() {
    for mood in MOODS {
        let notes = compose_with(mood, 7, 0.8);
        let bar = |bar: usize| {
            notes[2]
                .iter()
                .filter(|note| note.start / 16 == bar)
                .map(|note| (note.start % 16, note.len, note.pitch))
                .collect::<Vec<_>>()
        };
        // The question and the answer open alike, and the return repeats the theme.
        assert!(!bar(8).is_empty());
        assert_eq!(bar(8), bar(12), "{mood:?}");
        assert_eq!(bar(8), bar(24), "{mood:?}");
        assert_eq!(bar(9), bar(25), "{mood:?}");
    }
}

#[test]
fn the_bass_repeats_one_riff() {
    for mood in MOODS {
        let notes = compose_with(mood, 3, 0.8);
        let rhythm = |bar: usize| {
            notes[0]
                .iter()
                .filter(|note| note.start / 16 == bar)
                .map(|note| (note.start % 16, note.len))
                .collect::<Vec<_>>()
        };
        for bar in 9..15 {
            assert_eq!(rhythm(8), rhythm(bar), "{mood:?} bar {bar}");
        }
    }
}

#[test]
fn melodies_move_mostly_by_step() {
    for mood in MOODS {
        for seed in 0..32 {
            let lead = &compose_with(mood, seed, 0.8)[2];
            let leaps = lead
                .windows(2)
                .filter(|pair| (pair[1].pitch - pair[0].pitch).abs() > 7)
                .count();
            assert!(leaps * 10 <= lead.len(), "{mood:?} {seed}: {leaps} leaps");
        }
    }
}

#[test]
fn seeds_change_actual_melodic_pitches() {
    let first = compose(&Settings::default());
    let second = compose(&Settings {
        seed: 931,
        ..Settings::default()
    });
    let a: Vec<_> = first[2].iter().map(|n| n.pitch).collect();
    let b: Vec<_> = second[2].iter().map(|n| n.pitch).collect();
    assert_ne!(a, b);
}
