use super::*;

#[test]
fn exported_notes_round_trip_at_all_moods_and_keys() {
    for mood in [
        crate::desktop::Mood::Mysterious,
        crate::desktop::Mood::Hopeful,
        crate::desktop::Mood::Tense,
        crate::desktop::Mood::Melancholic,
    ] {
        for key in 0..12 {
            let settings = Settings {
                key,
                mood,
                ..Settings::default()
            };
            let score = crate::desktop::compose_with(&settings);
            let (imported, bpm) =
                read(&encode(&settings, &score).expect("encode")).expect("import");
            assert_eq!(score, imported);
            assert!((bpm - settings.bpm).abs() < 0.001);
        }
    }
}

fn file(events: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    chunk(&mut bytes, b"MThd", &[0, 0, 0, 1, 1, 224]);
    chunk(&mut bytes, b"MTrk", events);
    bytes
}

#[test]
fn reads_running_status_and_velocity_zero_note_off() {
    let (notes, _) = read(&file(&[
        0, 0x90, 60, 90, 120, 60, 0, 0, 62, 80, 120, 62, 0, 0, 255, 47, 0,
    ]))
    .expect("running status");
    assert_eq!(
        notes[0],
        vec![
            Note {
                pitch: 60,
                start: 0,
                len: 1,
                velocity: 90
            },
            Note {
                pitch: 62,
                start: 1,
                len: 1,
                velocity: 80
            }
        ]
    );
}

#[test]
fn rejects_polyphony_without_discarding_notes() {
    let result = read(&file(&[
        0, 0x90, 60, 90, 0, 64, 90, 120, 60, 0, 0, 64, 0, 0, 255, 47, 0,
    ]));
    assert!(result.expect_err("polyphony").contains("overlap"));
}

#[test]
fn rejects_truncated_files_and_invalid_quantities() {
    let settings = Settings::default();
    let bytes = encode(&settings, &crate::desktop::compose_with(&settings)).expect("encode");
    for length in 0..bytes.len() {
        assert!(read(&bytes[..length]).is_err(), "accepted prefix {length}");
    }
    assert!(read(&file(&[255, 255, 255, 255, 0])).is_err());
}

#[test]
fn rejects_unsupported_timing_and_performance_data() {
    assert!(read(&file(&[0, 0xe0, 0, 64])).is_err());
    assert!(read(&file(&[0, 0xb0, 64, 127])).is_err());
    assert!(read(&file(&[0, 0x99, 36, 100])).is_err());
    assert!(read(&file(&[0, 255, 88, 4, 3, 2, 24, 8])).is_err());
    assert!(read(&file(&[0, 255, 81, 3, 7, 161, 32, 1, 255, 81, 3, 6, 0, 0])).is_err());
}
