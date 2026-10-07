//! Copied instance joints bind the copy's root aliases through undo and removal.
use std::{collections::BTreeMap, time::Duration};

use serde_json::json;
use sindri_core::{
    CommandBuffer, CommandHistory, PrefabDocument, SceneDocument, SceneEntity, SceneEntityId,
    World, WorldCommand,
};
use sindri_scene::{SceneExtractor, ScenePhysics2d};

use super::duplicate_into;

#[test]
fn copied_joint_aliases_use_the_new_namespace_and_survive_undo() {
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
    let prefab: PrefabDocument = serde_json::from_value(json!({"format_version": 1, "entities": [
        {"id": "hook", "components": {"sindri.physics2d.collider": collider}},
        {"id": "body", "parent": "hook", "components": {
            "sindri.physics2d.collider": collider,
            "sindri.physics2d.rigid_body": body}},
        {"id": "joint", "parent": "hook", "components": {
            "sindri.physics2d.distance_joint": {"first": "hook", "second": "body", "max_distance": 2.0}}}
    ]})).unwrap();
    let prefabs = BTreeMap::from([("mechanism.prefab".into(), prefab)]);
    let id = |s| SceneEntityId::new(s).unwrap();
    let scene = SceneDocument {
        entities: vec![SceneEntity::instance(id("placed"), "mechanism.prefab")],
        ..SceneDocument::default()
    };
    let mut world = World::from_scene_with(&scene, &prefabs).unwrap().world;
    let root = world.entity_for_source_id(&id("placed")).unwrap();
    let mut buffer = CommandBuffer::new();
    let copied = duplicate_into(&mut world.clone(), &world, root, &mut buffer).unwrap();
    let mut history = CommandHistory::default();
    history
        .apply(buffer.into_transaction("Copy mechanism"), &mut world)
        .unwrap();
    let link = world.get(copied).unwrap().prefab.as_ref().unwrap();
    assert!(link.aliases.contains(&id("placed-copy/hook")));
    assert!(!link.aliases.contains(&id("placed/hook")));
    let registry = SceneExtractor::new().unwrap().components().clone();
    let mut physics = ScenePhysics2d::top_down().unwrap();
    let step = Duration::from_millis(16);
    physics.step(&mut world, &registry, step).unwrap();
    assert_eq!(physics.world().joint_count(), 2);
    let mut buffer = CommandBuffer::new();
    buffer.push(WorldCommand::Despawn { entity: root });
    history
        .apply(buffer.into_transaction("Remove original"), &mut world)
        .unwrap();
    physics.step(&mut world, &registry, step).unwrap();
    assert_eq!(physics.world().joint_count(), 1);
    history.undo(&mut world).unwrap();
    physics.step(&mut world, &registry, step).unwrap();
    assert_eq!(physics.world().joint_count(), 2);
}
