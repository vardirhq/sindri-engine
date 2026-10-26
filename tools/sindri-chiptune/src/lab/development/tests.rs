use super::*;
use crate::lab::{composer, midi, song::SongFile};

fn small_score() -> [Vec<Note>; 3] {
    [
        vec![
            Note {
                pitch: 38,
                start: 0,
                len: 16,
                velocity: 80,
            },
            Note {
                pitch: 41,
                start: 16,
                len: 16,
                velocity: 80,
            },
        ],
        vec![],
        vec![
            Note {
                pitch: 62,
                start: 0,
                len: 4,
                velocity: 90,
            },
            Note {
                pitch: 65,
                start: 8,
                len: 4,
                velocity: 80,
            },
        ],
    ]
}

#[test]
fn motifs_repeat_sequence_invert_and_stretch_without_overwriting_other_layers() {
    let settings = Settings::default();
    let tracks = small_score();
    let mut options = Motif {
        target: 1,
        shape: Shape::Repeat,
        follow_harmony: false,
        ..Motif::default()
    };
    let repeated = develop_motif(&settings, &tracks, 2, 0..1, &options).unwrap();
    assert_eq!(repeated[0], tracks[0]);
    assert_eq!(repeated[1], tracks[1]);
    assert_eq!(repeated[2][0..2], tracks[2]);
    assert_eq!(
        repeated[2][2],
        Note {
            start: 16,
            ..tracks[2][0]
        }
    );
    options.follow_harmony = true;
    let adapted = develop_motif(&settings, &tracks, 2, 0..1, &options).unwrap();
    assert_eq!(adapted[2][2].pitch, 65);
    options.follow_harmony = false;
    options.shape = Shape::Invert;
    let inverted = develop_motif(&settings, &tracks, 2, 0..1, &options).unwrap();
    assert_eq!(inverted[2][2].pitch, 62);
    assert!(inverted[2][3].pitch < inverted[2][2].pitch);
    options.shape = Shape::Sequence;
    options.steps = 1;
    options.stretch = 2;
    let developed = develop_motif(&settings, &tracks, 2, 0..1, &options).unwrap();
    assert_eq!(
        developed[2][2],
        Note {
            pitch: 64,
            start: 16,
            len: 8,
            velocity: 90
        }
    );
    assert_eq!(developed[2][3].start, 32);
    midi::validate(&developed).unwrap();
    options.target = 31;
    assert!(develop_motif(&settings, &tracks, 2, 0..1, &options).is_err());
    assert!(develop_motif(&settings, &tracks, 3, 0..1, &options).is_err());
    assert!(develop_motif(&settings, &tracks, 1, 0..1, &Motif::default()).is_err());
}

#[test]
fn dialogue_trades_phrases_with_bass_and_outside_material_preserved() {
    let settings = Settings::default();
    let tracks = composer::compose(&settings);
    for caller in [1, 2] {
        let options = Dialogue {
            caller,
            phrase_bars: 2,
        };
        let result = arrange_dialogue(&settings, &tracks, 8..12, &options).unwrap();
        assert_eq!(
            result,
            arrange_dialogue(&settings, &tracks, 8..12, &options).unwrap()
        );
        assert_eq!(result[0], tracks[0]);
        let responder = 3 - caller;
        let calls = clipped(&result[caller], &(128..192));
        let replies = clipped(&result[responder], &(128..192));
        assert!(!calls.is_empty() && !replies.is_empty());
        assert!(calls.iter().all(|call| {
            replies.iter().all(|reply| {
                call.start + call.len <= reply.start || reply.start + reply.len <= call.start
            })
        }));
        let last = replies.last().unwrap();
        assert_eq!(
            (last.pitch - crate::lab::phrases::root_at(&settings, &tracks[0], last.start).unwrap())
                .rem_euclid(12),
            0
        );
        for layer in [1, 2] {
            for note in &tracks[layer] {
                if note.start + note.len <= 128 || note.start >= 192 {
                    assert!(result[layer].contains(note));
                }
            }
        }
        midi::validate(&result).unwrap();
    }
    assert!(arrange_dialogue(&settings, &tracks, 8..11, &Dialogue::default()).is_err());
}

#[test]
fn expression_is_deterministic_bounded_and_preserves_pitches_and_onsets() {
    let settings = Settings::default();
    let notes = composer::compose(&settings)[2].clone();
    let neutral = Feel {
        strength: 0.0,
        gate: 100,
        ..Feel::default()
    };
    assert_eq!(
        shape_performance(&settings, &notes, 8..12, &neutral, 1).unwrap(),
        notes
    );
    let feel = Feel {
        strength: 1.0,
        gate: 50,
        accent: Accent::Backbeat,
    };
    let shaped = shape_performance(&settings, &notes, 8..12, &feel, 2).unwrap();
    assert_eq!(
        shaped,
        shape_performance(&settings, &notes, 8..12, &feel, 2).unwrap()
    );
    assert_ne!(shaped, notes);
    for (old, new) in notes.iter().zip(&shaped) {
        assert_eq!(old.pitch, new.pitch);
        assert_eq!(old.start, new.start);
        if old.start < 128 || old.start + old.len > 192 {
            assert_eq!(old, new);
        }
        assert!((1..=127).contains(&new.velocity));
        assert!((1..=old.len).contains(&new.len));
    }
    let crossing = [
        Note {
            pitch: 60,
            start: 120,
            len: 32,
            velocity: 80,
        },
        Note {
            pitch: 62,
            start: 152,
            len: 4,
            velocity: 80,
        },
    ];
    let shaped = shape_performance(&settings, &crossing, 8..10, &feel, 1).unwrap();
    assert_eq!(shaped[0], crossing[0]);
    let invalid = Feel {
        strength: f64::NAN,
        ..Feel::default()
    };
    assert!(shape_performance(&settings, &notes, 8..12, &invalid, 1).is_err());
}

#[test]
fn soundtrack_family_preserves_source_rhythm_mixes_and_project_compatibility() {
    let settings = Settings::default();
    let mut source = SongFile::new(&settings, &composer::compose(&settings));
    source.rack.arrangement.sections = [1, 3, 7, 31];
    source.rack.tracks[2].muted = true;
    let before = source.clone();
    let variants = family(&source);
    assert_eq!(variants, family(&source));
    assert_eq!(source, before);
    for (index, song) in variants.iter().enumerate() {
        assert_eq!(song.seed, source.seed);
        assert_eq!(song.key, source.key);
        assert_eq!(song.rack.arrangement, source.rack.arrangement);
        assert!(song.rack.tracks[2].muted);
        assert!(song.snapshots.is_empty());
        for (old, new) in source
            .tracks
            .iter()
            .flatten()
            .zip(song.tracks.iter().flatten())
        {
            assert_eq!(old.start, new.start);
            assert_eq!(old.len, new.len);
        }
        assert_eq!(
            song.mood,
            [Mood::Mysterious, Mood::Tense, Mood::Hopeful][index]
        );
        assert_eq!(*song, SongFile::from_json(&song.to_json()).unwrap());
    }
    assert_eq!(
        variants[2].tracks[0].last().unwrap().pitch.rem_euclid(12),
        source.key
    );
    assert_eq!(
        variants[2].tracks[2].last().unwrap().pitch.rem_euclid(12),
        source.key
    );
}

#[test]
fn development_stays_valid_across_moods_keys_and_seeds() {
    for mood in [
        Mood::Mysterious,
        Mood::Hopeful,
        Mood::Tense,
        Mood::Melancholic,
    ] {
        for key in [0, 2, 11] {
            for seed in 0..8 {
                let settings = Settings {
                    seed,
                    key,
                    mood,
                    ..Settings::default()
                };
                let tracks = composer::compose(&settings);
                midi::validate(
                    &develop_motif(&settings, &tracks, 2, 8..12, &Motif::default()).unwrap(),
                )
                .unwrap();
                midi::validate(
                    &arrange_dialogue(&settings, &tracks, 8..12, &Dialogue::default()).unwrap(),
                )
                .unwrap();
                for song in family(&SongFile::new(&settings, &tracks)) {
                    midi::validate(&song.tracks).unwrap();
                }
            }
        }
    }
}
