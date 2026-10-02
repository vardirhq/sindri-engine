use std::path::PathBuf;

use serde_json::json;
use sindri_core::{ComponentSchemaRegistry, EntityData, SceneComponent, World};
use sindri_decay::AudioCommand;
use sindri_platform::{AudioBackend, AudioError, SilentAudioBackend};
use sindri_scene::AudioSourceComponent;

use super::PlayAudio;

#[allow(clippy::unnecessary_wraps)] // An `Opener`, which may fail.
fn silent() -> Result<Box<dyn AudioBackend>, AudioError> {
    Ok(Box::new(SilentAudioBackend::default()))
}

/// A saved scene beside two clips, as a project lays them out.
fn project() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::create_dir_all(dir.path().join("audio")).expect("audio dir");
    for clip in ["theme.ogg", "shot.wav"] {
        std::fs::write(dir.path().join("audio").join(clip), [0_u8; 8]).expect("clip");
    }
    let scene = dir.path().join("main.scene");
    (dir, scene)
}

fn with_source(clip: &str, autoplay: bool, looping: bool) -> (World, ComponentSchemaRegistry) {
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<AudioSourceComponent>(AudioSourceComponent::TYPE_NAME)
        .expect("register");
    let mut world = World::default();
    world.spawn(EntityData {
        components: [(
            AudioSourceComponent::TYPE_NAME.to_owned(),
            json!({"clip": clip, "autoplay": autoplay, "looping": looping, "volume": 0.5}),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    (world, registry)
}

#[test]
fn an_autoplay_source_and_a_scripted_sound_both_play_on_their_buses() {
    let (_dir, scene) = project();
    let (world, registry) = with_source("audio/theme.ogg", true, true);
    let mut audio = PlayAudio::new(silent);
    assert!(audio.start(Some(&scene), &world, &registry).is_empty());
    let problems = audio.perform(vec![AudioCommand::Play {
        clip: "audio/shot.wav".to_owned(),
        volume: 1.0,
        bus: "effects".to_owned(),
    }]);
    assert!(problems.is_empty(), "{problems:?}");
    let playing: Vec<(String, String)> = audio
        .playing()
        .into_iter()
        .map(|voice| (voice.clip, voice.bus))
        .collect();
    assert_eq!(
        playing,
        [
            ("audio/theme.ogg".to_owned(), "music".to_owned()),
            ("audio/shot.wav".to_owned(), "effects".to_owned())
        ]
    );
    audio.stop();
    assert!(audio.playing().is_empty());
}

#[test]
fn a_missing_clip_is_reported_and_does_not_stop_the_rest() {
    let (_dir, scene) = project();
    let (world, registry) = with_source("audio/gone.ogg", true, false);
    let mut audio = PlayAudio::new(silent);
    let problems = audio.start(Some(&scene), &world, &registry);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("audio/gone.ogg"), "{problems:?}");
}

#[test]
fn the_monitor_mutes_and_solos_without_changing_what_the_game_set() {
    let (_dir, scene) = project();
    let (world, registry) = with_source("audio/theme.ogg", true, true);
    let mut audio = PlayAudio::new(silent);
    audio.start(Some(&scene), &world, &registry);
    audio.perform(vec![AudioCommand::SetVolume {
        bus: "music".to_owned(),
        volume: 0.4,
    }]);
    audio.set_solo("effects", true);
    let rows = audio.buses();
    let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, ["master", "effects", "music"], "master first");
    let music = rows.iter().find(|row| row.name == "music").expect("music");
    assert!((music.game - 0.4).abs() < f32::EPSILON);
    assert!(!music.audible, "another bus is soloed");
    assert!(rows[0].audible, "the master is never soloed away");
    assert!((audio.mixer.bus_volume("music")).abs() < f32::EPSILON);

    audio.set_solo("effects", false);
    audio.set_trim("music", 0.5);
    assert!((audio.mixer.bus_volume("music") - 0.2).abs() < 1e-6);
    audio.set_muted("music", true);
    assert!(audio.mixer.bus_volume("music").abs() < f32::EPSILON);

    // A new run starts from the game's own mix; the monitor carries over.
    audio.stop();
    let music = audio
        .buses()
        .into_iter()
        .find(|row| row.name == "music")
        .expect("music");
    assert!((music.game - 1.0).abs() < f32::EPSILON);
    assert!(music.muted);
}

#[allow(clippy::unnecessary_wraps)] // An `Opener` that fails.
fn no_device() -> Result<Box<dyn AudioBackend>, AudioError> {
    Err(AudioError::Output("no device here".to_owned()))
}

#[test]
fn without_a_device_play_goes_on_silently_and_one_shots_end() {
    let (_dir, scene) = project();
    let (world, registry) = with_source("audio/theme.ogg", true, true);
    let mut audio = PlayAudio::new(no_device);
    assert!(audio.start(Some(&scene), &world, &registry).is_empty());
    audio.perform(vec![AudioCommand::Play {
        clip: "audio/shot.wav".to_owned(),
        volume: 1.0,
        bus: "effects".to_owned(),
    }]);
    assert!(
        audio
            .unavailable()
            .is_some_and(|why| why.contains("no device"))
    );
    assert_eq!(audio.playing().len(), 2, "both listed while fresh");
    std::thread::sleep(super::SILENT_ONE_SHOT);
    let left: Vec<String> = audio
        .playing()
        .into_iter()
        .map(|voice| voice.clip)
        .collect();
    assert_eq!(
        left,
        ["audio/theme.ogg"],
        "the loop plays on, the shot is over"
    );
}
