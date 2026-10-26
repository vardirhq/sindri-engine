use super::*;
use crate::lab::{Generation, Mood, Settings, composer, midi, synth};

#[test]
fn rise_develops_the_theme_instead_of_inventing_an_unrelated_cell() {
    let theme = Bar(vec![(0, 4, 0), (4, 2, 2), (6, 2, 3), (8, 4, 4), (12, 4, 2)]);
    let rise = develop(&theme);
    assert_eq!(
        rise.0,
        vec![(0, 4, 2), (4, 2, 4), (6, 2, 5), (8, 4, 4), (12, 4, 6)]
    );
    assert_eq!(develop(&Bar(Vec::new())), Bar(Vec::new()));
}

#[test]
fn questions_stay_open_and_answers_prepare_then_reach_the_tonic() {
    for mood in [
        Mood::Hopeful,
        Mood::Melancholic,
        Mood::Mysterious,
        Mood::Tense,
    ] {
        for seed in 0..32 {
            let settings = Settings {
                seed,
                mood,
                ..Settings::default()
            };
            let plan = composer::plan(&settings);
            assert_ne!(chord(&plan, 7), 0);
            for end in [15, 23, 31] {
                assert_eq!(chord(&plan, end - 1), plan.style.dominant);
                assert_eq!(chord(&plan, end), 0);
            }
            let tracks = composer::compose(&settings);
            for end in [15, 23, 31] {
                let last = tracks[2].iter().rfind(|n| n.start / 16 == end).unwrap();
                assert_eq!((last.pitch - settings.key).rem_euclid(12), 0);
                assert!(
                    last.start + last.len < (end + 1) * 16,
                    "phrase needs a breath"
                );
            }
        }
    }
}

#[test]
fn busy_foreground_gets_room_without_disabling_layers() {
    let mut heard_busy = false;
    let mut reduced = false;
    for seed in 0..64 {
        let settings = Settings {
            seed,
            energy: 0.9,
            ..Settings::default()
        };
        let plan = composer::plan(&settings);
        let tracks = composer::compose(&settings);
        for bar in 0..32 {
            assert!(tracks[0].iter().any(|n| n.start / 16 == bar));
            assert!(tracks[1].iter().any(|n| n.start / 16 == bar));
            if tune(&plan, bar).is_some_and(|line| line.0.len() >= 6) {
                heard_busy = true;
                assert!(accompaniment_gap(&plan, bar, 1) >= 4);
                let arp: Vec<_> = tracks[1].iter().filter(|n| n.start / 16 == bar).collect();
                assert!(arp.len() <= 4);
                for note in arp {
                    if let Some(lead) = tracks[2]
                        .iter()
                        .find(|lead| lead.start <= note.start && lead.start + lead.len > note.start)
                    {
                        assert!(note.pitch <= lead.pitch - 3, "register collision at {bar}");
                    }
                }
            }
        }
        let legacy = composer::compose(&Settings {
            generation: Generation::Legacy,
            ..settings
        });
        reduced |= tracks[1].len() < legacy[1].len();
    }
    assert!(heard_busy && reduced);
}

#[test]
fn both_versions_remain_seeded_valid_varied_and_transposable() {
    for generation in [Generation::Legacy, Generation::Musical] {
        for mood in [
            Mood::Hopeful,
            Mood::Melancholic,
            Mood::Mysterious,
            Mood::Tense,
        ] {
            for seed in 0..32 {
                for key in [0, 5, 11] {
                    let settings = Settings {
                        seed,
                        generation,
                        key,
                        energy: 0.85,
                        mood,
                        ..Settings::default()
                    };
                    let tracks = composer::compose(&settings);
                    assert_eq!(tracks, composer::compose(&settings));
                    midi::validate(&tracks).unwrap();
                    for note in tracks.iter().flatten() {
                        assert!(synth::scale(mood).contains(&(note.pitch - key).rem_euclid(12)));
                    }
                    for track in 0..3 {
                        let mut varied = tracks.clone();
                        varied[track] = composer::compose_track(&settings, track, 7);
                        midi::validate(&varied).unwrap();
                        assert_eq!(varied[track], composer::compose_track(&settings, track, 7));
                        assert_ne!(varied[track], tracks[track]);
                    }
                }
            }
        }
    }
}

/// Frozen from the composer on main before Musical was introduced. Each field
/// is hashed as a little-endian u64; no platform-dependent serialization hash.
#[test]
fn legacy_scores_keep_their_original_fingerprint() {
    let mut fingerprint = 0xcbf2_9ce4_8422_2325_u64;
    for mood in [
        Mood::Hopeful,
        Mood::Melancholic,
        Mood::Mysterious,
        Mood::Tense,
    ] {
        for seed in [0, 1, 42, 931, u64::MAX] {
            let settings = Settings {
                seed,
                generation: Generation::Legacy,
                mood,
                ..Settings::default()
            };
            for track in composer::compose(&settings) {
                for note in track {
                    for value in [
                        u64::try_from(note.pitch).unwrap(),
                        u64::try_from(note.start).unwrap(),
                        u64::try_from(note.len).unwrap(),
                        u64::from(note.velocity),
                    ] {
                        for byte in value.to_le_bytes() {
                            fingerprint =
                                (fingerprint ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
                        }
                    }
                }
            }
        }
    }
    assert_eq!(fingerprint, 0xd099_bea6_2299_ba97);
}
