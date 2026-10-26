use super::*;
use std::collections::BTreeSet;

#[test]
fn ensembles_are_seeded_diverse_and_leave_room_for_silence() {
    let mut palettes = BTreeSet::new();
    let mut plans = BTreeSet::new();
    let mut omitted = false;
    for seed in 0..64 {
        let settings = Settings {
            seed,
            ..Settings::default()
        };
        let rack = generate(&settings);
        assert_eq!(rack, generate(&settings));
        rack.arrangement.validate().unwrap();
        assert_eq!(rack.arrangement.version, VERSION);
        assert!(rack.arrangement.sections.iter().all(|mask| *mask != 0));
        assert!(rack.arrangement.active(1, 2) && rack.arrangement.active(2, 2));
        assert!(rack.arrangement.sections[0].count_ones() <= 2);
        assert!(
            rack.arrangement.sections[2].count_ones() >= rack.arrangement.sections[1].count_ones()
        );
        palettes.insert(format!("{:?}", rack.tracks.map(|sound| sound.preset)));
        plans.insert(rack.arrangement.sections);
        omitted |=
            (0..5).any(|layer| (0..4).all(|section| !rack.arrangement.active(section, layer)));
    }
    assert_eq!(palettes.len(), 3);
    assert_eq!(plans.len(), 4);
    assert!(
        omitted,
        "some ensembles should omit a layer for the entire song"
    );
}

#[test]
fn version_one_seed_42_is_a_stable_reference() {
    let rack = generate(&Settings::default());
    assert_eq!(rack.arrangement.sections, [16, 21, 21, 5]);
    assert_eq!(rack.channel(0).preset, Preset::WarmTriangle);
    assert_eq!(rack.channel(1).preset, Preset::Glass);
    assert_eq!(rack.channel(2).preset, Preset::HollowPulse);
    assert_eq!(rack.channel(3).preset, Preset::SoftNoise);
    assert_eq!(rack.channel(4).preset, Preset::Detuned);
}

#[test]
fn arrangement_clips_cross_section_notes_and_legacy_keeps_them() {
    let note = Note {
        start: 120,
        len: 144,
        pitch: 60,
        velocity: 80,
    };
    let legacy = Arrangement::default();
    assert_eq!(legacy.arrange(2, &[note]), vec![note]);
    let plan = Arrangement {
        version: VERSION,
        sections: [4, 0, 4, 0],
    };
    assert_eq!(
        plan.arrange(2, &[note]),
        vec![
            Note { len: 8, ..note },
            Note {
                start: 256,
                len: 8,
                ..note
            }
        ]
    );
    assert!(plan.arrange(0, &[note]).is_empty());
    let mut broken = plan;
    broken.sections[1] = 32;
    assert!(broken.validate().is_err());
    broken.sections[1] = 0;
    broken.version = VERSION + 1;
    assert!(broken.validate().is_err());
}

#[test]
fn midi_exports_section_rests_without_destroying_saved_notes() {
    let mut settings = Settings::default();
    settings.rack.arrangement.sections = [4, 0, 4, 0];
    let tracks = [
        vec![],
        vec![],
        vec![Note {
            start: 120,
            len: 144,
            pitch: 60,
            velocity: 80,
        }],
    ];
    let bytes = crate::lab::midi::encode(&settings, &tracks).unwrap();
    let (imported, _) = crate::lab::midi::read(&bytes).unwrap();
    assert_eq!(
        imported.into_iter().flatten().collect::<Vec<_>>(),
        settings.rack.arrangement.arrange(2, &tracks[2])
    );
    let song = crate::lab::song::SongFile::new(&settings, &tracks);
    assert_eq!(
        crate::lab::song::SongFile::from_json(&song.to_json())
            .unwrap()
            .tracks,
        tracks
    );
}

#[test]
fn old_songs_keep_their_mix_and_new_plans_are_checked_on_load() {
    let settings = Settings::default();
    let tracks = crate::lab::compose_with(&settings);
    let song = crate::lab::song::SongFile::new(&settings, &tracks);
    let mut old = serde_json::to_value(&song).unwrap();
    old["rack"].as_object_mut().unwrap().remove("arrangement");
    let opened = crate::lab::song::SongFile::from_json(&old.to_string()).unwrap();
    assert_eq!(opened.rack, settings.rack);
    assert_eq!(opened.tracks, tracks);
    old["rack"]["arrangement"] = serde_json::json!({"version": 99, "sections": [31, 31, 31, 31]});
    assert!(crate::lab::song::SongFile::from_json(&old.to_string()).is_err());
}
