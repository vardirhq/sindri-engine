//! Prefab reload updates reference aliases through the same undoable commands.
use std::{collections::BTreeMap, time::Duration};

use serde_json::json;
use sindri_core::{CommandBuffer, CommandHistory, PrefabDocument, SceneEntityId, World};
use sindri_scene::{SceneExtractor, ScenePhysics2d};

use super::{reconcile, spawn_instance};

fn library(root: &str) -> BTreeMap<String, PrefabDocument> {
    let registry = SceneExtractor::new().unwrap().components().clone();
    let collider = registry
        .default_payload("sindri.physics2d.collider")
        .unwrap()
        .clone();
    let mut body = registry
        .default_payload("sindri.physics2d.rigid_body")
        .unwrap()
        .clone();
    body["kind"] = json!("dynamic");
    let inner = serde_json::from_value(json!({"format_version": 1, "entities": [
        {"id": root, "components": {"sindri.physics2d.collider": collider}},
        {"id": "body", "parent": root, "components": {
            "sindri.physics2d.collider": collider,
            "sindri.physics2d.rigid_body": body}},
        {"id": "joint", "parent": root, "components": {
            "sindri.physics2d.distance_joint": {"first": root, "second": "body", "max_distance": 2.0}}}
    ]})).unwrap();
    let outer = serde_json::from_value(json!({"format_version": 1, "entities": [
        {"id": "assembly"}, {"id": "mechanism", "parent": "assembly", "prefab": {"source": "inner.prefab"}}
    ]})).unwrap();
    BTreeMap::from([
        ("inner.prefab".into(), inner),
        ("outer.prefab".into(), outer),
    ])
}

#[test]
fn placed_joint_aliases_reload_and_undo_without_replacing_bodies() {
    let before = library("hook");
    let after = library("renamed");
    let mut world = World::default();
    let mut rehearsal = world.clone();
    let mut buffer = CommandBuffer::new();
    let root = spawn_instance(
        &mut rehearsal,
        "outer.prefab",
        &before,
        None,
        None,
        &mut buffer,
    )
    .unwrap();
    let mut history = CommandHistory::default();
    history
        .apply(buffer.into_transaction("Place mechanism"), &mut world)
        .unwrap();
    let id = |s| SceneEntityId::new(s).unwrap();
    let hook = world
        .entity_for_source_id(&id("assembly/mechanism"))
        .unwrap();
    let moving = world
        .entity_for_source_id(&id("assembly/mechanism/body"))
        .unwrap();
    let aliases = |world: &World| {
        world
            .get(hook)
            .unwrap()
            .prefab
            .as_ref()
            .unwrap()
            .aliases
            .clone()
    };
    assert!(aliases(&world).contains(&id("assembly/mechanism/hook")));
    let registry = SceneExtractor::new().unwrap().components().clone();
    let mut physics = ScenePhysics2d::top_down().unwrap();
    let step = Duration::from_millis(16);
    physics.step(&mut world, &registry, step).unwrap();
    assert_eq!(physics.world().joint_count(), 1);
    let placed = world.instance_entity(root, &before).unwrap();
    let mut buffer = CommandBuffer::new();
    reconcile(
        &world,
        &mut world.clone(),
        &world.instance_members(root),
        &placed,
        &after,
        &mut buffer,
    )
    .unwrap();
    history
        .apply(buffer.into_transaction("Reload roots"), &mut world)
        .unwrap();
    assert!(aliases(&world).contains(&id("assembly/mechanism/renamed")));
    assert!(!aliases(&world).contains(&id("assembly/mechanism/hook")));
    physics.step(&mut world, &registry, step).unwrap();
    assert_eq!(physics.world().joint_count(), 1);
    assert!(physics.world().contains(moving));
    history.undo(&mut world).unwrap();
    assert!(aliases(&world).contains(&id("assembly/mechanism/hook")));
    physics.step(&mut world, &registry, step).unwrap();
    assert_eq!(physics.world().joint_count(), 1);
    assert!(physics.world().contains(moving));
    history.redo(&mut world).unwrap();
    assert!(aliases(&world).contains(&id("assembly/mechanism/renamed")));
    physics.step(&mut world, &registry, step).unwrap();
    assert_eq!(physics.world().joint_count(), 1);
}
