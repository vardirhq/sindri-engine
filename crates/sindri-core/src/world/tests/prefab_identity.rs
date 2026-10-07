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
    assert_eq!(
        world.prefab_entity(owner, "root"),
        Some(spawned.by_source_id[&id("nested")])
    );
    assert_eq!(
        world.clone().prefab_entity(owner, "root"),
        Some(spawned.by_source_id[&id("nested")])
    );
    assert_eq!(world.prefab_entity(owner, "outer"), Some(spawned.root));
    assert_eq!(
        world.prefab_entity(owner, "nested"),
        Some(spawned.by_source_id[&id("nested")])
    );
}

#[test]
fn canonical_paths_shadow_aliases_and_ambiguous_aliases_fail_before_spawn() {
    let inner: PrefabDocument = serde_json::from_value(json!({"format_version": 1,
        "entities": [{"id": "root"}, {"id": "joint", "parent": "root"}]}))
    .unwrap();
    let other: PrefabDocument = serde_json::from_value(json!({"format_version": 1,
        "entities": [{"id": "branch/root"}]}))
    .unwrap();
    let library = std::collections::BTreeMap::from([
        ("inner.prefab".into(), inner),
        ("other.prefab".into(), other),
    ]);
    let mut outer: PrefabDocument = serde_json::from_value(json!({"format_version": 1,
        "entities": [{"id": "outer"},
            {"id": "nested", "parent": "outer", "prefab": {"source": "inner.prefab"}},
            {"id": "nested/root", "parent": "outer", "disabled": true}]}))
    .unwrap();
    let mut world = World::default();
    let spawned = world.spawn_prefab_from(&outer, &library).unwrap();
    let id = |s| SceneEntityId::new(s).unwrap();
    let owner = spawned.by_source_id[&id("nested/joint")];
    assert_eq!(
        world.prefab_entity(owner, "root"),
        Some(spawned.by_source_id[&id("nested/root")])
    );
    outer.entities = serde_json::from_value(json!([
        {"id": "outer"},
        {"id": "nested", "parent": "outer", "prefab": {"source": "other.prefab"}},
        {"id": "nested/branch", "parent": "outer", "prefab": {"source": "inner.prefab"}}
    ]))
    .unwrap();
    let before = world.entities().count();
    assert!(matches!(world.spawn_prefab_from(&outer, &library),
        Err(crate::WorldError::InvalidPrefab(crate::PrefabError::AmbiguousRootAlias(alias)))
        if alias == id("nested/branch/root")));
    assert_eq!(world.entities().count(), before);
    assert_eq!(
        world.prefab_entity(owner, "root"),
        Some(spawned.by_source_id[&id("nested/root")])
    );
}

#[test]
fn repeated_expansion_and_undo_preserve_nested_root_aliases() {
    let leaf: PrefabDocument = serde_json::from_value(json!({"format_version": 1,
        "entities": [{"id": "anchor"}, {"id": "joint", "parent": "anchor"}]}))
    .unwrap();
    let middle: PrefabDocument = serde_json::from_value(json!({"format_version": 1,
        "entities": [{"id": "mount"},
            {"id": "inner", "parent": "mount", "prefab": {"source": "leaf.prefab"}}]}))
    .unwrap();
    let outer: PrefabDocument = serde_json::from_value(json!({"format_version": 1,
        "entities": [{"id": "assembly"},
            {"id": "mechanism", "parent": "assembly", "prefab": {"source": "middle.prefab"}}]}))
    .unwrap();
    let library = std::collections::BTreeMap::from([
        ("leaf.prefab".into(), leaf),
        ("middle.prefab".into(), middle),
    ]);
    let mut world = World::default();
    let first = world.spawn_prefab_from(&outer, &library).unwrap();
    let second = world.spawn_prefab_from(&outer, &library).unwrap();
    let id = |s| SceneEntityId::new(s).unwrap();
    let owner = first.by_source_id[&id("mechanism/inner/joint")];
    let anchor = first.by_source_id[&id("mechanism/inner")];
    assert_eq!(world.prefab_entity(owner, "anchor"), Some(anchor));
    assert_eq!(
        world.prefab_entity(owner, "mount"),
        Some(first.by_source_id[&id("mechanism")])
    );
    assert_ne!(
        world.prefab_entity(owner, "anchor"),
        Some(second.by_source_id[&id("mechanism/inner")])
    );
    let mut buffer = CommandBuffer::new();
    buffer.push(WorldCommand::Despawn { entity: first.root });
    let mut history = CommandHistory::default();
    history
        .apply(
            buffer.into_transaction("Remove nested mechanism"),
            &mut world,
        )
        .unwrap();
    history.undo(&mut world).unwrap();
    assert_eq!(world.prefab_entity(owner, "anchor"), Some(anchor));
}
