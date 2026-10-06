//! Runtime-local paths do not become saved identities or escape their spawn.
use serde_json::json;

use crate::{
    CommandBuffer, CommandHistory, EntityData, PrefabDocument, SceneEntityId, World, WorldCommand,
};

fn prefab() -> PrefabDocument {
    serde_json::from_value(json!({"format_version": 1, "entities": [
        {"id": "root"},
        {"id": "body", "parent": "root"},
        {"id": "nested", "parent": "root"},
        {"id": "nested/body", "parent": "nested"},
        {"id": "nested/joint", "parent": "nested"}
    ]}))
    .unwrap()
}

#[test]
fn paths_are_local_even_after_reparenting_and_assigning_saved_ids() {
    let mut world = World::default();
    let first = world.spawn_prefab(&prefab()).unwrap();
    let second = world.spawn_prefab(&prefab()).unwrap();
    let id = |s| SceneEntityId::new(s).unwrap();
    let owner = first.by_source_id[&id("nested/joint")];
    let body = first.by_source_id[&id("nested/body")];
    assert_eq!(world.prefab_entity(owner, "body"), Some(body));
    assert_eq!(world.prefab_entity(owner, "root"), Some(first.root));
    assert_eq!(world.prefab_entity(owner, "missing"), None);
    assert_eq!(world.prefab_entity(owner, ""), None);
    assert!(
        world
            .entities()
            .all(|(_, data)| data.source_id.is_none() && data.prefab.is_none())
    );
    world.set_parent(owner, Some(second.root)).unwrap();
    world.set_parent(body, Some(second.root)).unwrap();
    world.assign_missing_source_ids("saved").unwrap();
    assert_eq!(world.prefab_entity(owner, "body"), Some(body));
    assert_eq!(world.clone().prefab_entity(owner, "root"), Some(first.root));
    let restored = World::from_scene(&world.to_scene().unwrap()).unwrap().world;
    assert!(
        restored
            .entities()
            .all(|(_, data)| data.prefab_identity.is_none())
    );
    world.despawn_recursive(first.root).unwrap();
    world.spawn(EntityData::default());
    assert_eq!(
        world.prefab_entity(owner, "body"),
        None,
        "a reused root slot is a different spawn"
    );
}

#[test]
fn command_despawn_and_undo_restore_local_identity() {
    let mut world = World::default();
    let spawned = world.spawn_prefab(&prefab()).unwrap();
    let owner = spawned.by_source_id[&SceneEntityId::new("nested/joint").unwrap()];
    let mut buffer = CommandBuffer::new();
    buffer.push(WorldCommand::Despawn {
        entity: spawned.root,
    });
    let mut history = CommandHistory::default();
    history
        .apply(buffer.into_transaction("Remove spawn"), &mut world)
        .unwrap();
    assert_eq!(world.prefab_entity(owner, "root"), None);
    history.undo(&mut world).unwrap();
    assert_eq!(world.prefab_entity(owner, "root"), Some(spawned.root));
}

#[test]
fn expanded_nested_siblings_keep_their_local_namespace() {
    let inner: PrefabDocument = serde_json::from_value(json!({"format_version": 1,
        "entities": [{"id": "root"}, {"id": "body", "parent": "root"},
            {"id": "joint", "parent": "root"}]}))
    .unwrap();
    let outer: PrefabDocument = serde_json::from_value(json!({"format_version": 1,
        "entities": [{"id": "outer"}, {"id": "body", "parent": "outer"},
            {"id": "nested", "parent": "outer", "prefab": {"source": "inner.prefab"}}]}))
    .unwrap();
    let library = std::collections::BTreeMap::from([("inner.prefab".into(), inner)]);
    let mut world = World::default();
    let spawned = world.spawn_prefab_from(&outer, &library).unwrap();
    let id = |s| SceneEntityId::new(s).unwrap();
    let owner = spawned.by_source_id[&id("nested/joint")];
    assert_eq!(
        world.prefab_entity(owner, "body"),
        Some(spawned.by_source_id[&id("nested/body")])
    );
    assert_eq!(world.prefab_entity(owner, "outer"), Some(spawned.root));
    assert_eq!(
        world.prefab_entity(owner, "nested"),
        Some(spawned.by_source_id[&id("nested")])
    );
}
