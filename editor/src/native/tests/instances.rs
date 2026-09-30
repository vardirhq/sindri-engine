//! Editing an instance's shape: copying it, and renaming it.
//!
//! An instance's entities are named by their place under its root, and that
//! is what it is saved by. Copying and renaming are the two edits that change
//! names, so each has to keep that true or the scene stops saving.

use std::collections::BTreeMap;

use sindri_core::{
    CommandBuffer, CommandHistory, PrefabDocument, SceneDocument, SceneEntity, SceneEntityId, World,
};

use super::super::editing::duplicate::duplicate_into;
use super::super::inspector_panel::draft::{IdentityRefusal, identity_commands};

const COIN: &str = "prefabs/coin.prefab";

fn id(value: &str) -> SceneEntityId {
    SceneEntityId::new(value).unwrap()
}

fn prefabs() -> BTreeMap<String, PrefabDocument> {
    let coin = PrefabDocument {
        entities: vec![
            SceneEntity::new(id("coin")),
            SceneEntity {
                parent: Some(id("coin")),
                ..SceneEntity::new(id("sparkle"))
            },
        ],
        ..PrefabDocument::default()
    };
    BTreeMap::from([(COIN.to_owned(), coin)])
}

fn world() -> World {
    let scene = SceneDocument {
        entities: vec![SceneEntity::instance(id("coin-1"), COIN)],
        ..SceneDocument::default()
    };
    World::from_scene_with(&scene, &prefabs()).unwrap().world
}

fn run(world: &mut World, buffer: CommandBuffer) {
    CommandHistory::default()
        .apply(buffer.into_transaction("test"), world)
        .unwrap();
}

#[test]
fn a_copied_instance_is_another_instance_of_the_same_prefab() {
    let mut world = world();
    let root = world.entity_for_source_id(&id("coin-1")).unwrap();
    let mut rehearsal = world.clone();
    let mut buffer = CommandBuffer::new();
    duplicate_into(&mut rehearsal, &world.clone(), root, &mut buffer).unwrap();
    run(&mut world, buffer);

    let saved = world.to_scene_with(&prefabs()).unwrap();
    assert_eq!(saved.entities.len(), 2, "two instances: {saved:?}");
    assert!(saved.entities.iter().all(|entity| entity.prefab.is_some()));
}

#[test]
fn part_of_an_instance_copied_alone_is_plain_entities() {
    let mut world = world();
    let sparkle = world.entity_for_source_id(&id("coin-1/sparkle")).unwrap();
    let mut rehearsal = world.clone();
    let mut buffer = CommandBuffer::new();
    let copy = duplicate_into(&mut rehearsal, &world.clone(), sparkle, &mut buffer).unwrap();
    run(&mut world, buffer);

    assert_eq!(world.get(copy).unwrap().prefab, None);
    let saved = world.to_scene_with(&prefabs()).unwrap();
    assert_eq!(
        saved.entities.len(),
        2,
        "the instance, and the copied sparkle"
    );
}

#[test]
fn renaming_an_instance_renames_what_is_under_it() {
    let mut world = world();
    let root = world.entity_for_source_id(&id("coin-1")).unwrap();
    let buffer = identity_commands(&world, root, "lucky-coin").unwrap();
    run(&mut world, buffer);
    assert!(
        world
            .entity_for_source_id(&id("lucky-coin/sparkle"))
            .is_some()
    );
    world.to_scene_with(&prefabs()).expect("it still saves");

    let sparkle = world
        .entity_for_source_id(&id("lucky-coin/sparkle"))
        .unwrap();
    assert_eq!(
        identity_commands(&world, sparkle, "glint").err(),
        Some(IdentityRefusal::InInstance)
    );
}
