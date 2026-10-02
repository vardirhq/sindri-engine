use serde_json::json;
use sindri_core::{ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, World};

use super::{
    Cue, CueSound, Key, Property, Sequence, SequenceComponent, SequenceError, Sequences, Track,
    pose,
};

fn key(time: f32, value: f32) -> Key {
    Key {
        time,
        value,
        ease: "linear".to_owned(),
    }
}

fn registry() -> ComponentSchemaRegistry {
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<SequenceComponent>("Sequence")
        .expect("register");
    registry
}

/// A director with a child called Logo, carrying `sequence` as "intro".
fn stage(sequence: &Sequence, playing: bool) -> (World, EntityId, EntityId) {
    let mut world = World::default();
    let director = world.spawn(EntityData {
        name: Some("Director".to_owned()),
        components: [(
            SequenceComponent::TYPE_NAME.to_owned(),
            json!({
                "sequences": { "intro": sequence },
                "playing": if playing { json!("intro") } else { json!(null) },
            }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let logo = world.spawn(EntityData {
        name: Some("Logo".to_owned()),
        components: [(
            "sindri.sprite".to_owned(),
            json!({ "tint": [1.0, 1.0, 1.0, 0.0] }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    world.set_parent(logo, Some(director)).expect("parent");
    (world, director, logo)
}

fn slide() -> Sequence {
    Sequence {
        duration: 2.0,
        looping: false,
        tracks: vec![
            Track {
                target: "Logo".to_owned(),
                property: "position.x".to_owned(),
                keys: vec![key(0.0, -10.0), key(1.0, 0.0)],
            },
            Track {
                target: "Logo".to_owned(),
                property: "sindri.sprite/tint.3".to_owned(),
                keys: vec![key(0.5, 0.0), key(1.5, 1.0)],
            },
        ],
        cues: vec![
            Cue {
                time: 0.0,
                name: "begin".to_owned(),
                sound: None,
            },
            Cue {
                time: 1.0,
                name: "landed".to_owned(),
                sound: Some(CueSound {
                    clip: "audio/thud.wav".to_owned(),
                    bus: String::new(),
                    volume: 1.0,
                }),
            },
            Cue {
                time: 2.0,
                name: "end".to_owned(),
                sound: None,
            },
        ],
    }
}

fn x_of(world: &World, entity: EntityId) -> f32 {
    world
        .get(entity)
        .and_then(|data| data.transform_3d)
        .map_or(f32::NAN, |transform| transform.position[0])
}

#[test]
fn a_track_holds_its_ends_and_eases_between_keys() {
    let mut track = Track {
        target: String::new(),
        property: "position.x".to_owned(),
        keys: vec![key(1.0, 0.0), key(3.0, 10.0)],
    };
    assert_eq!(track.sample(0.0), Some(0.0));
    assert_eq!(track.sample(2.0), Some(5.0));
    assert_eq!(track.sample(9.0), Some(10.0));
    track.keys[0].ease = "ease-in".to_owned();
    let eased = track.sample(2.0).expect("value");
    assert!(eased < 5.0, "ease-in starts slow: {eased}");
    assert_eq!(Track::default().sample(1.0), None);
}

#[test]
fn properties_read_and_write_the_transform_and_component_fields() {
    let mut data = EntityData {
        components: [(
            "sindri.sprite".to_owned(),
            json!({ "tint": [1.0, 1.0, 1.0, 1.0] }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    };
    for (text, value) in [
        ("position.y", 3.0),
        ("rotation", 90.0),
        ("scale", 2.0),
        ("sindri.sprite/tint.3", 0.25),
    ] {
        let property = Property::parse(text).expect(text);
        property.write(&mut data, value).expect(text);
        let read = property.read(&data).expect(text);
        assert!((read - value).abs() < 1e-4, "{text}: {read}");
    }
    let transform = data.transform_3d.expect("written");
    assert!((transform.scale[1] - 2.0).abs() < 1e-6 && (transform.scale[2] - 1.0).abs() < 1e-6);
    for bad in ["position.w", "spin", "sindri.sprite/", "/tint"] {
        assert!(Property::parse(bad).is_err(), "{bad}");
    }
    assert_eq!(
        Property::parse("sindri.sprite/missing.0")
            .expect("parses")
            .write(&mut data, 1.0),
        Err(SequenceError::Field("sindri.sprite/missing.0".to_owned()))
    );
}

#[test]
fn a_played_sequence_moves_its_child_and_reaches_each_cue_once() {
    let (mut world, director, logo) = stage(&slide(), true);
    let registry = registry();
    let mut sequences = Sequences::new();

    let step = sequences
        .advance(&mut world, &registry, 0.5)
        .expect("advance");
    assert!(step.problems.is_empty(), "{:?}", step.problems);
    let names: Vec<&str> = step.cues.iter().map(|cue| cue.name.as_str()).collect();
    assert_eq!(
        names,
        ["begin"],
        "the start's cue is reached on the first step"
    );
    assert!((x_of(&world, logo) + 5.0).abs() < 1e-4);
    assert!(sequences.cued(director, "begin"));

    let step = sequences
        .advance(&mut world, &registry, 0.5)
        .expect("advance");
    let names: Vec<&str> = step.cues.iter().map(|cue| cue.name.as_str()).collect();
    assert_eq!(names, ["landed"]);
    assert_eq!(step.sounds.len(), 1);
    assert_eq!(step.sounds[0].bus(), "effects");
    assert!(!sequences.cued(director, "begin"), "a cue is for one step");

    let step = sequences
        .advance(&mut world, &registry, 5.0)
        .expect("advance");
    let names: Vec<&str> = step.cues.iter().map(|cue| cue.name.as_str()).collect();
    assert_eq!(names, ["end"], "the end's cue is reached as it finishes");
    assert!(sequences.is_finished(director));
    assert_eq!(sequences.time(director), Some(2.0));
    let alpha = world.get(logo).expect("logo").components["sindri.sprite"]["tint"][3]
        .as_f64()
        .expect("number");
    assert!((alpha - 1.0).abs() < 1e-6);

    // Finished, it lets go: a script may move the logo afterwards.
    world.get_mut(logo).expect("logo").transform_3d = None;
    sequences
        .advance(&mut world, &registry, 0.5)
        .expect("advance");
    assert!(world.get(logo).expect("logo").transform_3d.is_none());
}

#[test]
fn a_looping_sequence_wraps_and_reaches_its_cues_again() {
    let mut sequence = slide();
    sequence.looping = true;
    let (mut world, director, _) = stage(&sequence, true);
    let registry = registry();
    let mut sequences = Sequences::new();
    sequences
        .advance(&mut world, &registry, 1.5)
        .expect("advance");
    let step = sequences
        .advance(&mut world, &registry, 1.0)
        .expect("advance");
    let names: Vec<&str> = step.cues.iter().map(|cue| cue.name.as_str()).collect();
    assert_eq!(names, ["end", "begin"]);
    assert!(!sequences.is_finished(director));
    assert!((sequences.time(director).expect("playing") - 0.5).abs() < 1e-5);
}

#[test]
fn nothing_plays_unless_named_and_a_broken_one_is_said_once() {
    let (mut world, director, logo) = stage(&slide(), false);
    let registry = registry();
    let mut sequences = Sequences::new();
    sequences
        .advance(&mut world, &registry, 0.5)
        .expect("advance");
    assert_eq!(sequences.time(director), None);
    assert!(world.get(logo).expect("logo").transform_3d.is_none());

    let mut broken = slide();
    broken.tracks[0].property = "spin".to_owned();
    let (mut world, _, _) = stage(&broken, true);
    let first = sequences
        .advance(&mut world, &registry, 0.1)
        .expect("advance");
    assert_eq!(first.problems.len(), 1);
    let again = sequences
        .advance(&mut world, &registry, 0.1)
        .expect("advance");
    assert!(again.problems.is_empty(), "said once, not every step");
}

#[test]
fn pose_shows_any_moment_without_playing() {
    let (mut world, director, logo) = stage(&slide(), false);
    assert!(pose(&mut world, director, &slide(), 0.25).is_empty());
    assert!((x_of(&world, logo) + 7.5).abs() < 1e-4);
    let mut lost = slide();
    lost.tracks[0].target = "Nobody".to_owned();
    let problems = pose(&mut world, director, &lost, 0.25);
    assert_eq!(problems, [SequenceError::Target("Nobody".to_owned())]);
}
