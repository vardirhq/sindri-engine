use sindri_core::{EntityData, World};
use sindri_scene::{Sequence, SequenceComponent};

use super::{Edit, Picked, TimelineState, apply, current_value, snapped, targets};

fn component() -> SequenceComponent {
    SequenceComponent {
        sequences: [(
            "intro".to_owned(),
            Sequence {
                duration: 2.0,
                ..Sequence::default()
            },
        )]
        .into_iter()
        .collect(),
        playing: None,
        speed: 1.0,
    }
}

#[test]
fn keys_stay_in_time_order_through_adds_and_moves() {
    let mut sequence = component();
    apply(
        &mut sequence,
        "intro",
        Edit::AddTrack {
            target: "Logo".to_owned(),
            property: "position.x".to_owned(),
        },
    );
    for (time, value) in [(1.0, 10.0), (0.0, 0.0), (0.5, 5.0)] {
        apply(
            &mut sequence,
            "intro",
            Edit::Key {
                track: 0,
                time,
                value,
            },
        );
    }
    let times = |sequence: &SequenceComponent| -> Vec<f32> {
        sequence.sequences["intro"].tracks[0]
            .keys
            .iter()
            .map(|key| key.time)
            .collect()
    };
    assert_eq!(times(&sequence), [0.0, 0.5, 1.0]);

    // Keying an existing time sets it rather than adding a second key there.
    let picked = apply(
        &mut sequence,
        "intro",
        Edit::Key {
            track: 0,
            time: 0.5,
            value: 7.0,
        },
    );
    assert_eq!(picked, Some(Picked::Key { track: 0, key: 1 }));
    assert_eq!(times(&sequence).len(), 3);

    // Dragged past its neighbour, the key is re-sorted and stays picked.
    let picked = apply(
        &mut sequence,
        "intro",
        Edit::MoveKey {
            track: 0,
            key: 0,
            time: 1.5,
        },
    );
    assert_eq!(times(&sequence), [0.5, 1.0, 1.5]);
    assert_eq!(picked, Some(Picked::Key { track: 0, key: 2 }));
    assert!(
        sequence.sequences["intro"].check("intro").is_ok(),
        "what the panel writes is what the runtime accepts"
    );
}

#[test]
fn cues_sequences_and_autoplay_are_edited_in_place() {
    let mut sequence = component();
    assert_eq!(
        apply(&mut sequence, "intro", Edit::AddCue { time: 0.5 }),
        Some(Picked::Cue(0))
    );
    apply(
        &mut sequence,
        "intro",
        Edit::SetCue {
            cue: 0,
            time: 0.75,
            name: "landed".to_owned(),
            sound: "audio/thud.wav".to_owned(),
        },
    );
    let cue = &sequence.sequences["intro"].cues[0];
    assert_eq!(cue.name, "landed");
    assert_eq!(
        cue.sound.as_ref().map(|sound| sound.clip.as_str()),
        Some("audio/thud.wav")
    );
    apply(
        &mut sequence,
        "intro",
        Edit::SetCue {
            cue: 0,
            time: 0.75,
            name: "landed".to_owned(),
            sound: String::new(),
        },
    );
    assert!(sequence.sequences["intro"].cues[0].sound.is_none());

    apply(
        &mut sequence,
        "intro",
        Edit::AddSequence("outro".to_owned()),
    );
    apply(
        &mut sequence,
        "intro",
        Edit::Autoplay(Some("outro".to_owned())),
    );
    assert_eq!(sequence.playing.as_deref(), Some("outro"));
    let mut state = TimelineState::default();
    let (shown, _) = state.chosen(&sequence).expect("one is shown");
    assert_eq!(shown, "outro", "the playing one is shown first");
}

#[test]
fn the_playhead_snaps_wraps_and_stops() {
    assert!((snapped(0.73, 2.0) - 0.75).abs() < 1e-5);
    assert!((snapped(9.0, 2.0) - 2.0).abs() < 1e-5);
    let mut state = TimelineState {
        playing: true,
        time: 1.9,
        ..TimelineState::default()
    };
    let mut sequence = Sequence {
        duration: 2.0,
        looping: true,
        ..Sequence::default()
    };
    state.tick(&sequence, 0.2);
    assert!((state.time - 0.1).abs() < 1e-5 && state.playing);
    sequence.looping = false;
    state.tick(&sequence, 5.0);
    assert!((state.time - 2.0).abs() < 1e-5 && !state.playing);
}

#[test]
fn targets_are_the_carrier_and_its_named_descendants() {
    let mut world = World::default();
    let carrier = world.spawn(EntityData::default());
    let ship = world.spawn(EntityData {
        name: Some("Ship".to_owned()),
        ..EntityData::default()
    });
    let flame = world.spawn(EntityData {
        name: Some("Flame".to_owned()),
        ..EntityData::default()
    });
    let unnamed = world.spawn(EntityData::default());
    world.set_parent(ship, Some(carrier)).expect("parent");
    world.set_parent(flame, Some(ship)).expect("parent");
    world.set_parent(unnamed, Some(carrier)).expect("parent");
    let camera = world.spawn(EntityData {
        name: Some("Camera".to_owned()),
        ..EntityData::default()
    });
    world.get_mut(carrier).expect("carrier").name = Some("Director".to_owned());
    assert_eq!(
        targets(&world, carrier),
        ["", "Ship", "Ship/Flame", "/Camera"],
        "relative paths for its own, a scene path for the rest"
    );
    let _ = camera;

    let track = sindri_scene::Track {
        target: "Ship/Flame".to_owned(),
        property: "scale".to_owned(),
        keys: Vec::new(),
    };
    assert_eq!(current_value(&world, carrier, &track), Some(1.0));
}
