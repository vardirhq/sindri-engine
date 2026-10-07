//! Picker scope and diagnostics follow the same rules as runtime references.
use super::entities::{EntityReferences, ReferenceStatus};
use serde_json::json;
use sindri_core::{LoadedScenes, NoPrefabs, PrefabDocument, SceneDocument, SceneEntityId, World};

#[test]
fn loaded_scene_choices_and_inactive_local_diagnostics_never_cross_roots() {
    let document: SceneDocument = serde_json::from_value(
        json!({"format_version":sindri_core::SCENE_FORMAT_VERSION,"entities":[
            {"id":"assembly/owner", "components":{"test":{"target":"body"}}},
            {"id":"assembly/body", "name":"Local body", "disabled":true},
            {"id":"body", "name":"External body"}
        ]}),
    )
    .unwrap();
    let mut world = World::default();
    let mut scenes = LoadedScenes::new();
    scenes
        .enter_with(&mut world, "one", &document, &NoPrefabs)
        .unwrap();
    scenes
        .load_with(&mut world, "two", &document, &NoPrefabs)
        .unwrap();
    let owner = world
        .entity_for_source_id(&SceneEntityId::new("one/assembly/owner").unwrap())
        .unwrap();
    let references = EntityReferences::new(&world, owner, &world.get(owner).unwrap().components);
    assert_eq!(
        references.status("body"),
        ReferenceStatus::Inactive("Local body".into())
    );
    assert_eq!(references.status(""), ReferenceStatus::Unbound);
    assert_eq!(references.status("missing"), ReferenceStatus::Missing);
    assert_eq!(references.choices.len(), 3);
    assert!(
        references
            .choices
            .iter()
            .all(|choice| !choice.reference.starts_with("two/"))
    );
    for choice in &references.choices {
        assert!(
            world
                .resolve_entity_reference(owner, &choice.reference)
                .is_some()
        );
    }
}

#[test]
fn spawned_prefab_choices_stay_local_after_assigning_saved_ids() {
    let prefab: PrefabDocument = serde_json::from_value(json!({"format_version":1,"entities":[
        {"id":"root"}, {"id":"owner", "parent":"root", "components":{"test":{"nested":[{"target":"body"}]}}},
        {"id":"body", "parent":"root", "name":"Local body"}
    ]})).unwrap();
    let mut world = World::default();
    let spawn = world.spawn_prefab(&prefab).unwrap();
    world.spawn_prefab(&prefab).unwrap();
    world.assign_missing_source_ids("saved").unwrap();
    let owner = spawn.by_source_id[&SceneEntityId::new("owner").unwrap()];
    let references = EntityReferences::new(&world, owner, &world.get(owner).unwrap().components);
    assert_eq!(
        references
            .choices
            .iter()
            .map(|choice| choice.reference.as_str())
            .collect::<Vec<_>>(),
        ["body", "owner", "root"]
    );
    assert_eq!(
        references.status("body"),
        ReferenceStatus::Active("Local body".into())
    );
    world.despawn_recursive(spawn.root).unwrap();
    let references = EntityReferences::new(&world, owner, &std::collections::BTreeMap::default());
    assert!(references.choices.is_empty());
    assert_eq!(references.status("body"), ReferenceStatus::Missing);
}
