//! A run's edits, kept and discarded, on the platformer.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sindri_core::{CommandBuffer, CommandHistory, EntityData, EntityId, World, WorldCommand};

use super::{RunEdits, Verdict, review};
use crate::native::{load_world, scene_extractor};
use crate::scene_file::SceneFile;

const SCRIPT: &str = "sindri.script";

/// The platformer's assets copied somewhere a test may save, and its scene.
fn platformer() -> (PathBuf, PathBuf) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../games/platformer/assets");
    let target = std::env::temp_dir().join(format!(
        "sindri-run-edits-{}-{}",
        std::process::id(),
        std::thread::current()
            .name()
            .unwrap_or("test")
            .replace("::", "-")
    ));
    copy(&source, &target);
    let scene = target.join("platformer.scene");
    (target, scene)
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a folder");
    for entry in std::fs::read_dir(from).expect("the assets read").flatten() {
        let path = entry.path();
        let into = to.join(entry.file_name());
        if path.is_dir() {
            copy(&path, &into);
        } else {
            std::fs::copy(&path, &into).expect("a file copies");
        }
    }
}

fn hero(world: &World) -> EntityId {
    world
        .entities()
        .find(|(_, data)| {
            data.source_id
                .as_ref()
                .map(sindri_core::SceneEntityId::as_str)
                == Some("hero")
        })
        .map(|(entity, _)| entity)
        .expect("the platformer has its hero")
}

fn script(world: &World, entity: EntityId) -> Value {
    world
        .get(entity)
        .and_then(|data| data.components.get(SCRIPT))
        .cloned()
        .expect("the hero has a script")
}

/// The inspector's edit of one exported property, as one transaction.
fn tune(world: &World, entity: EntityId, property: &str, value: f64) -> sindri_core::Transaction {
    let mut payload = script(world, entity);
    payload["properties"][property] = json!(value);
    let mut commands = CommandBuffer::new();
    commands.push(WorldCommand::SetComponent {
        entity,
        type_name: SCRIPT.to_owned(),
        payload,
    });
    commands.into_transaction(format!("Set {property}"))
}

/// Plays `scene`: the run world is a copy of the snapshot, as Play makes it.
/// Tunes the jump while it plays, and lets "the game" write another property
/// of the same component, as a script writing its own state would.
fn play_and_tune(snapshot: &World) -> (World, RunEdits) {
    let mut running = snapshot.clone();
    let mut edits = RunEdits::default();
    let hero = hero(&running);
    edits
        .apply(tune(&running, hero, "jump_speed", 18.0), &mut running)
        .expect("the edit applies to the running world");
    let mut written = script(&running, hero);
    written["properties"]["speed"] = json!(99.0);
    running
        .get_mut(hero)
        .expect("the hero")
        .components
        .insert(SCRIPT.to_owned(), written);
    (running, edits)
}

#[test]
fn a_jump_tuned_while_playing_and_kept_is_in_the_saved_scene_and_nothing_else_is() {
    let (folder, path) = platformer();
    let extractor = scene_extractor();
    let mut file = SceneFile::open(&path).expect("the scene opens");
    let snapshot = load_world(&extractor, &file).expect("the scene loads");

    let (running, mut edits) = play_and_tune(&snapshot);
    assert_eq!(
        script(&running, hero(&running))["properties"]["jump_speed"],
        json!(18.0),
        "the running hero jumps higher at once"
    );

    // Stop: the snapshot comes back, and the edit is offered.
    let mut world = snapshot;
    let edits = edits.take();
    let verdicts = review(&edits, &world);
    let mut history = CommandHistory::default();
    for verdict in verdicts {
        let Verdict::Keep(transaction) = verdict else {
            panic!("the jump can be kept");
        };
        history
            .apply(transaction, &mut world)
            .expect("kept as an ordinary history entry");
    }
    assert!(history.can_undo(), "keeping it is undoable");
    file.save(&world, extractor.components()).expect("saves");

    let reopened = SceneFile::open(&path).expect("reopens");
    let saved = load_world(&extractor, &reopened).expect("loads");
    let properties = &script(&saved, hero(&saved))["properties"];
    assert_eq!(
        properties["jump_speed"],
        json!(18.0),
        "the kept jump is saved"
    );
    assert!(
        properties.get("speed").is_none(),
        "what the game wrote while it ran is not: {properties}"
    );
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn a_jump_tuned_while_playing_and_discarded_leaves_the_scene_as_it_was() {
    let (folder, path) = platformer();
    let extractor = scene_extractor();
    let mut file = SceneFile::open(&path).expect("the scene opens");
    let snapshot = load_world(&extractor, &file).expect("the scene loads");
    let before = script(&snapshot, hero(&snapshot));

    let (_running, mut edits) = play_and_tune(&snapshot);
    // Stop, discarding everything.
    drop(edits.take());
    file.save(&snapshot, extractor.components()).expect("saves");

    let reopened = SceneFile::open(&path).expect("reopens");
    let saved = load_world(&extractor, &reopened).expect("loads");
    assert_eq!(script(&saved, hero(&saved)), before);
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn an_edit_to_something_the_run_spawned_is_explained_not_dropped() {
    let snapshot = World::default();
    let mut running = snapshot.clone();
    let spawned = running.spawn(EntityData {
        name: Some("Bullet".to_owned()),
        ..EntityData::default()
    });
    let mut edits = RunEdits::default();
    let mut commands = CommandBuffer::new();
    commands.push(WorldCommand::SetName {
        entity: spawned,
        name: Some("Slow bullet".to_owned()),
    });
    edits
        .apply(commands.into_transaction("Rename"), &mut running)
        .expect("applies while playing");
    let verdicts = review(&edits.take(), &snapshot);
    let [Verdict::Explained(why)] = verdicts.as_slice() else {
        panic!("explained: {verdicts:?}");
    };
    assert!(
        why.contains("Bullet") && why.contains("made while the scene played"),
        "{why}"
    );
}

#[test]
fn a_drag_while_playing_is_one_edit() {
    let (folder, path) = platformer();
    let extractor = scene_extractor();
    let file = SceneFile::open(&path).expect("the scene opens");
    let snapshot = load_world(&extractor, &file).expect("the scene loads");
    let mut running = snapshot.clone();
    let hero = hero(&running);
    let mut edits = RunEdits::default();
    for step in 0..10 {
        let transaction =
            tune(&running, hero, "jump_speed", 14.0 + f64::from(step)).merging("drag");
        edits.apply(transaction, &mut running).expect("applies");
    }
    edits.break_merge_run();
    edits
        .apply(
            tune(&running, hero, "gravity", 20.0).merging("drag"),
            &mut running,
        )
        .expect("applies");
    assert_eq!(
        edits.len(),
        2,
        "the drag, then a new edit after the release"
    );
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn a_scale_changed_while_the_run_moved_the_hero_keeps_the_scale_not_the_move() {
    let (folder, path) = platformer();
    let extractor = scene_extractor();
    let file = SceneFile::open(&path).expect("the scene opens");
    let snapshot = load_world(&extractor, &file).expect("the scene loads");
    let hero = hero(&snapshot);
    let authored = snapshot
        .get(hero)
        .and_then(|data| data.transform_3d)
        .expect("a transform");

    let mut running = snapshot.clone();
    // The run moves the hero.
    running
        .get_mut(hero)
        .expect("hero")
        .transform_3d
        .as_mut()
        .expect("transform")
        .position[0] += 5.0;
    // The person doubles its size in the inspector, which writes the whole
    // transform as the running world holds it.
    let mut scaled = running
        .get(hero)
        .and_then(|data| data.transform_3d)
        .expect("transform");
    scaled.scale = [2.0, 2.0, 1.0];
    let mut commands = CommandBuffer::new();
    commands.push(WorldCommand::SetTransform3D {
        entity: hero,
        transform: Some(scaled),
    });
    let mut edits = RunEdits::default();
    edits
        .apply(commands.into_transaction("Scale"), &mut running)
        .expect("applies");

    let mut world = snapshot;
    let verdicts = review(&edits.take(), &world);
    let [Verdict::Keep(transaction)] = verdicts.as_slice() else {
        panic!("the scale can be kept: {verdicts:?}");
    };
    let transaction = transaction.clone();
    CommandHistory::default()
        .apply(transaction, &mut world)
        .expect("kept");
    let kept = world
        .get(hero)
        .and_then(|data| data.transform_3d)
        .expect("transform");
    assert_eq!(
        kept.scale.map(f32::to_bits),
        [2.0_f32, 2.0, 1.0].map(f32::to_bits),
        "the scale is kept"
    );
    assert_eq!(
        kept.position.map(f32::to_bits),
        authored.position.map(f32::to_bits),
        "where the run moved it is not"
    );
    let _ = std::fs::remove_dir_all(folder);
}
