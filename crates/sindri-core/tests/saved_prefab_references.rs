//! Registry meanings remap entity references, never arbitrary strings.
use serde::Deserialize;
use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, FieldMeaning, NoPrefabs, PrefabDocument, SceneComponent,
    SceneEntityId, World, WorldError,
};

#[derive(Deserialize)]
struct Links {
    targets: Vec<Target>,
}
#[derive(Deserialize)]
struct Target {
    entity: String,
}
impl SceneComponent for Links {
    const TYPE_NAME: &'static str = "test.links";
}

fn registry() -> ComponentSchemaRegistry {
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register_with_fields::<Links>("Links", json!({"targets": [{"entity": ""}]}))
        .unwrap();
    registry
        .describe::<Links>([("targets[].entity", FieldMeaning::Entity)])
        .unwrap();
    registry
}

fn prefab() -> PrefabDocument {
    serde_json::from_value(json!({"format_version": 1, "entities": [
        {"id": "root"}, {"id": "body", "parent": "root", "disabled": true},
        {"id": "owner", "parent": "root", "components": {
            "test.links": {"targets": [{"entity": "root"}, {"entity": "body"}, {"entity": ""}], "label": "body"},
            "unknown": {"entity": "body"}}}
    ]})).unwrap()
}

#[test]
fn list_references_remap_without_changing_unknown_fields_or_live_state() {
    let mut world = World::default();
    let first = world.spawn_prefab(&prefab()).unwrap();
    world.spawn_prefab(&prefab()).unwrap();
    world.assign_missing_source_ids("saved").unwrap();
    let owner = first.by_source_id[&SceneEntityId::new("owner").unwrap()];
    let data = world.get(owner).unwrap().clone();
    let saved = world
        .to_scene_with_references(&NoPrefabs, &registry())
        .unwrap();
    let payload = &saved
        .entity(data.source_id.as_ref().unwrap())
        .unwrap()
        .components;
    assert_eq!(
        payload["test.links"]["targets"][0]["entity"],
        world
            .get(first.root)
            .unwrap()
            .source_id
            .as_ref()
            .unwrap()
            .as_str()
    );
    assert_eq!(
        payload["test.links"]["targets"][1]["entity"],
        world
            .get(first.by_source_id[&SceneEntityId::new("body").unwrap()])
            .unwrap()
            .source_id
            .as_ref()
            .unwrap()
            .as_str()
    );
    assert_eq!(payload["test.links"]["targets"][2]["entity"], "");
    assert_eq!(payload["test.links"]["label"], "body");
    assert_eq!(payload["unknown"]["entity"], "body");
    assert_eq!(world.get(owner), Some(&data));
    let links: Links = serde_json::from_value(payload["test.links"].clone()).unwrap();
    assert!(!links.targets[0].entity.is_empty());
}

#[test]
fn unresolved_unstable_and_wrong_type_references_fail_without_live_mutation() {
    let mut world = World::default();
    let spawn = world.spawn_prefab(&prefab()).unwrap();
    assert!(matches!(
        world.to_scene_with_references(&NoPrefabs, &registry()),
        Err(WorldError::UnstableEntity(_))
    ));
    world.assign_missing_source_ids("saved").unwrap();
    let owner = spawn.by_source_id[&SceneEntityId::new("owner").unwrap()];
    for target in [json!("missing"), json!(42)] {
        world
            .get_mut(owner)
            .unwrap()
            .components
            .get_mut("test.links")
            .unwrap()["targets"][0]["entity"] = target;
        let before = world.get(owner).unwrap().clone();
        assert!(matches!(
            world.to_scene_with_references(&NoPrefabs, &registry()),
            Err(WorldError::InvalidPrefabReference { .. })
        ));
        assert_eq!(world.get(owner), Some(&before));
    }
    world
        .get_mut(owner)
        .unwrap()
        .components
        .insert("test.links".into(), json!({"targets": [42]}));
    let before = world.get(owner).unwrap().clone();
    assert!(matches!(
        world.to_scene_with_references(&NoPrefabs, &registry()),
        Err(WorldError::InvalidPrefabReference { .. })
    ));
    assert_eq!(world.get(owner), Some(&before));
}
