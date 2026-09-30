//! A scene placing prefabs as instances: what they load into, and what they
//! save back as.

use std::collections::BTreeMap;

use serde_json::json;
use sindri_core::{
    EntityOverride, PrefabDocument, PrefabError, PrefabInstance, PrefabLink, SceneDocument,
    SceneEntity, SceneEntityId, SceneError, Transform3D, World, WorldError,
};

fn id(name: &str) -> SceneEntityId {
    SceneEntityId::new(name).unwrap()
}

fn at(x: f32, y: f32) -> Transform3D {
    Transform3D {
        position: [x, y, 0.0],
        ..Transform3D::default()
    }
}

/// A coin: a sprite at the root, and a sparkle under it.
fn coin() -> PrefabDocument {
    let mut root = SceneEntity {
        name: Some("Coin".to_owned()),
        transform_3d: Some(at(0.0, 0.0)),
        ..SceneEntity::new(id("coin"))
    };
    root.components.insert(
        "sindri.sprite".to_owned(),
        json!({ "texture": "textures/coin.png", "layer": 2 }),
    );
    root.components
        .insert("sindri.tags".to_owned(), json!({ "tags": ["coin"] }));
    let mut sparkle = SceneEntity {
        name: Some("Sparkle".to_owned()),
        parent: Some(id("coin")),
        transform_3d: Some(at(0.0, 0.5)),
        ..SceneEntity::new(id("sparkle"))
    };
    sparkle.components.insert(
        "sindri.sprite".to_owned(),
        json!({ "texture": "textures/sparkle.png", "layer": 3 }),
    );
    PrefabDocument {
        entities: vec![root, sparkle],
        ..PrefabDocument::default()
    }
}

fn library(prefabs: &[(&str, PrefabDocument)]) -> BTreeMap<String, PrefabDocument> {
    prefabs
        .iter()
        .map(|(source, prefab)| ((*source).to_owned(), prefab.clone()))
        .collect()
}

fn instance(name: &str, source: &str, place: Transform3D) -> SceneEntity {
    SceneEntity {
        transform_3d: Some(place),
        ..SceneEntity::instance(id(name), source)
    }
}

fn scene(entities: Vec<SceneEntity>) -> SceneDocument {
    let mut document = SceneDocument {
        entities,
        ..SceneDocument::default()
    };
    document.canonicalize();
    document
}

fn component(world: &World, source_id: &str, type_name: &str) -> serde_json::Value {
    let entity = world.entity_for_source_id(&id(source_id)).unwrap();
    world.get(entity).unwrap().components[type_name].clone()
}

#[test]
fn an_instance_loads_as_the_entities_its_prefab_describes() {
    let prefabs = library(&[("prefabs/coin.prefab", coin())]);
    let document = scene(vec![instance(
        "coin-1",
        "prefabs/coin.prefab",
        at(4.0, 2.0),
    )]);
    let world = World::from_scene_with(&document, &prefabs).unwrap().world;

    assert_eq!(world.len(), 2);
    let root = world.entity_for_source_id(&id("coin-1")).unwrap();
    let sparkle = world.entity_for_source_id(&id("coin-1/sparkle")).unwrap();
    let root_data = world.get(root).unwrap();
    assert_eq!(root_data.name.as_deref(), Some("Coin"));
    assert_eq!(root_data.transform_3d, Some(at(4.0, 2.0)));
    assert_eq!(
        root_data.prefab,
        Some(PrefabLink {
            source: "prefabs/coin.prefab".to_owned(),
            path: id("coin"),
            root: true,
        })
    );
    let sparkle_data = world.get(sparkle).unwrap();
    assert_eq!(sparkle_data.parent, Some(root));
    assert_eq!(sparkle_data.prefab.as_ref().unwrap().path, id("sparkle"));
    assert_eq!(world.instance_root(sparkle), Some(root));
    assert_eq!(world.instance_members(root), vec![root, sparkle]);
}

#[test]
fn two_instances_of_one_prefab_do_not_collide() {
    let prefabs = library(&[("prefabs/coin.prefab", coin())]);
    let document = scene(vec![
        instance("coin-1", "prefabs/coin.prefab", at(1.0, 0.0)),
        instance("coin-2", "prefabs/coin.prefab", at(2.0, 0.0)),
    ]);
    let world = World::from_scene_with(&document, &prefabs).unwrap().world;
    assert_eq!(world.len(), 4);
    assert!(world.entity_for_source_id(&id("coin-2/sparkle")).is_some());
}

#[test]
fn an_unedited_instance_saves_as_the_reference_it_was() {
    let prefabs = library(&[("prefabs/coin.prefab", coin())]);
    let document = scene(vec![instance(
        "coin-1",
        "prefabs/coin.prefab",
        at(4.0, 2.0),
    )]);
    let world = World::from_scene_with(&document, &prefabs).unwrap().world;
    assert_eq!(world.to_scene_with(&prefabs).unwrap(), document);
}

#[test]
fn an_edit_to_an_instance_saves_as_an_override_and_reloads() {
    let prefabs = library(&[("prefabs/coin.prefab", coin())]);
    let document = scene(vec![instance(
        "coin-1",
        "prefabs/coin.prefab",
        at(4.0, 2.0),
    )]);
    let mut world = World::from_scene_with(&document, &prefabs).unwrap().world;
    let sparkle = world.entity_for_source_id(&id("coin-1/sparkle")).unwrap();
    world.get_mut(sparkle).unwrap().components.insert(
        "sindri.sprite".to_owned(),
        json!({ "texture": "textures/sparkle.png", "layer": 5 }),
    );
    let root = world.entity_for_source_id(&id("coin-1")).unwrap();
    world
        .get_mut(root)
        .unwrap()
        .components
        .remove("sindri.tags");

    let saved = world.to_scene_with(&prefabs).unwrap();
    let written = saved.entity(&id("coin-1")).unwrap();
    assert!(
        written.components.is_empty(),
        "an instance holds no components"
    );
    let overrides = &written.prefab.as_ref().unwrap().overrides;
    assert_eq!(
        overrides[&id("sparkle")].components["sindri.sprite"],
        json!({ "layer": 5 })
    );
    assert_eq!(
        overrides[&id("coin")].components["sindri.tags"],
        serde_json::Value::Null
    );
    assert_eq!(saved.entities.len(), 1);

    let reloaded = World::from_scene_with(&saved, &prefabs).unwrap().world;
    assert_eq!(
        component(&reloaded, "coin-1/sparkle", "sindri.sprite"),
        json!({ "texture": "textures/sparkle.png", "layer": 5 })
    );
    let root = reloaded.entity_for_source_id(&id("coin-1")).unwrap();
    assert!(
        !reloaded
            .get(root)
            .unwrap()
            .components
            .contains_key("sindri.tags")
    );
}

#[test]
fn putting_a_value_back_is_no_longer_an_override() {
    let prefabs = library(&[("prefabs/coin.prefab", coin())]);
    let mut placed = instance("coin-1", "prefabs/coin.prefab", at(0.0, 0.0));
    let mut changes = EntityOverride::default();
    changes
        .components
        .insert("sindri.sprite".to_owned(), json!({ "layer": 9 }));
    placed
        .prefab
        .as_mut()
        .unwrap()
        .overrides
        .insert(id("coin"), changes);
    let mut world = World::from_scene_with(&scene(vec![placed]), &prefabs)
        .unwrap()
        .world;
    let root = world.entity_for_source_id(&id("coin-1")).unwrap();
    world.get_mut(root).unwrap().components.insert(
        "sindri.sprite".to_owned(),
        json!({ "texture": "textures/coin.png", "layer": 2 }),
    );
    let saved = world.to_scene_with(&prefabs).unwrap();
    assert!(
        saved.entities[0]
            .prefab
            .as_ref()
            .unwrap()
            .overrides
            .is_empty()
    );
}

#[test]
fn a_changed_prefab_reaches_its_instances_but_not_their_overrides() {
    let mut placed = instance("coin-1", "prefabs/coin.prefab", at(0.0, 0.0));
    let mut changes = EntityOverride::default();
    changes
        .components
        .insert("sindri.sprite".to_owned(), json!({ "layer": 9 }));
    placed
        .prefab
        .as_mut()
        .unwrap()
        .overrides
        .insert(id("coin"), changes);
    let document = scene(vec![placed]);

    let mut edited = coin();
    edited.entities[0].components.insert(
        "sindri.sprite".to_owned(),
        json!({ "texture": "textures/gold.png", "layer": 2 }),
    );
    let prefabs = library(&[("prefabs/coin.prefab", edited)]);
    let world = World::from_scene_with(&document, &prefabs).unwrap().world;
    assert_eq!(
        component(&world, "coin-1", "sindri.sprite"),
        json!({ "texture": "textures/gold.png", "layer": 9 })
    );
}

#[test]
fn a_scene_can_hang_its_own_entities_under_an_instance() {
    let prefabs = library(&[("prefabs/coin.prefab", coin())]);
    let extra = SceneEntity {
        parent: Some(id("coin-1/sparkle")),
        ..SceneEntity::new(id("glint"))
    };
    let document = scene(vec![
        instance("coin-1", "prefabs/coin.prefab", at(0.0, 0.0)),
        extra,
    ]);
    document.validate().unwrap();
    let world = World::from_scene_with(&document, &prefabs).unwrap().world;
    let glint = world.entity_for_source_id(&id("glint")).unwrap();
    let sparkle = world.entity_for_source_id(&id("coin-1/sparkle")).unwrap();
    assert_eq!(world.get(glint).unwrap().parent, Some(sparkle));
    // It is the scene's, not the instance's, and saves as itself.
    assert_eq!(world.to_scene_with(&prefabs).unwrap(), document);

    let stray = SceneEntity {
        parent: Some(id("coin-1/nothing")),
        ..SceneEntity::new(id("stray"))
    };
    let document = scene(vec![
        instance("coin-1", "prefabs/coin.prefab", at(0.0, 0.0)),
        stray,
    ]);
    assert!(matches!(
        World::from_scene_with(&document, &prefabs),
        Err(WorldError::InvalidScene(SceneError::MissingParent { .. }))
    ));
}

#[test]
fn prefabs_nest_and_an_override_reaches_inside_the_inner_one() {
    let mut chest = SceneEntity::new(id("chest"));
    chest.components.insert(
        "sindri.sprite".to_owned(),
        json!({ "texture": "chest.png" }),
    );
    let inner_coin = SceneEntity {
        parent: Some(id("chest")),
        transform_3d: Some(at(0.0, 1.0)),
        ..SceneEntity::instance(id("loot"), "prefabs/coin.prefab")
    };
    let chest_prefab = PrefabDocument {
        entities: vec![chest, inner_coin],
        ..PrefabDocument::default()
    };
    let prefabs = library(&[
        ("prefabs/coin.prefab", coin()),
        ("prefabs/chest.prefab", chest_prefab.clone()),
    ]);

    let mut placed = instance("chest-1", "prefabs/chest.prefab", at(3.0, 0.0));
    let mut changes = EntityOverride::default();
    changes
        .components
        .insert("sindri.sprite".to_owned(), json!({ "layer": 7 }));
    placed
        .prefab
        .as_mut()
        .unwrap()
        .overrides
        .insert(id("loot/sparkle"), changes);
    let document = scene(vec![placed]);
    let world = World::from_scene_with(&document, &prefabs).unwrap().world;
    assert_eq!(
        world.len(),
        3,
        "the chest, and the coin inside it with its sparkle"
    );
    assert_eq!(
        component(&world, "chest-1/loot/sparkle", "sindri.sprite"),
        json!({ "texture": "textures/sparkle.png", "layer": 7 })
    );
    let loot = world.entity_for_source_id(&id("chest-1/loot")).unwrap();
    assert_eq!(world.get(loot).unwrap().transform_3d, Some(at(0.0, 1.0)));
    assert_eq!(world.to_scene_with(&prefabs).unwrap(), document);

    // And a script's spawn makes the nested one too.
    let mut spawned = World::default();
    let created = spawned.spawn_prefab_from(&chest_prefab, &prefabs).unwrap();
    assert_eq!(created.entities.len(), 3);
    assert!(created.by_source_id.contains_key(&id("loot/sparkle")));
    assert!(matches!(
        World::default().spawn_prefab(&chest_prefab),
        Err(WorldError::InvalidPrefab(PrefabError::Missing(_)))
    ));
}

#[test]
fn a_prefab_that_contains_itself_is_refused() {
    let looping = PrefabDocument::single(SceneEntity::instance(id("again"), "prefabs/loop.prefab"));
    let prefabs = library(&[("prefabs/loop.prefab", looping)]);
    let document = scene(vec![instance("x", "prefabs/loop.prefab", at(0.0, 0.0))]);
    assert!(matches!(
        World::from_scene_with(&document, &prefabs),
        Err(WorldError::InvalidPrefab(PrefabError::Cycle(_)))
    ));
}

#[test]
fn an_instance_needs_its_prefab() {
    let document = scene(vec![instance(
        "coin-1",
        "prefabs/coin.prefab",
        at(0.0, 0.0),
    )]);
    assert!(matches!(
        World::from_scene(&document),
        Err(WorldError::InvalidPrefab(PrefabError::Missing(source))) if source == "prefabs/coin.prefab"
    ));
}

#[test]
fn an_instance_carries_no_components_of_its_own() {
    let mut placed = instance("coin-1", "prefabs/coin.prefab", at(0.0, 0.0));
    placed
        .components
        .insert("sindri.sprite".to_owned(), json!({}));
    assert_eq!(
        scene(vec![placed]).validate(),
        Err(SceneError::InstanceComponents(id("coin-1")))
    );
}

#[test]
fn an_instance_missing_an_entity_cannot_be_written_as_a_reference() {
    let prefabs = library(&[("prefabs/coin.prefab", coin())]);
    let document = scene(vec![instance(
        "coin-1",
        "prefabs/coin.prefab",
        at(0.0, 0.0),
    )]);
    let mut world = World::from_scene_with(&document, &prefabs).unwrap().world;
    let sparkle = world.entity_for_source_id(&id("coin-1/sparkle")).unwrap();
    world.despawn_recursive(sparkle).unwrap();
    assert!(matches!(
        world.to_scene_with(&prefabs),
        Err(WorldError::InstanceReshaped { .. })
    ));
}

#[test]
fn an_unpacked_instance_saves_as_plain_entities() {
    let prefabs = library(&[("prefabs/coin.prefab", coin())]);
    let document = scene(vec![instance(
        "coin-1",
        "prefabs/coin.prefab",
        at(0.0, 0.0),
    )]);
    let mut world = World::from_scene_with(&document, &prefabs).unwrap().world;
    let members = world.instance_members(world.entity_for_source_id(&id("coin-1")).unwrap());
    for entity in members {
        world.get_mut(entity).unwrap().prefab = None;
    }
    let saved = world.to_scene().unwrap();
    assert_eq!(saved.entities.len(), 2);
    assert!(saved.entities.iter().all(|entity| entity.prefab.is_none()));
}

#[test]
fn an_instance_reads_the_way_it_is_written() {
    let json = r#"{
  "format_version": 10,
  "entities": [
    {
      "id": "coin-1",
      "transform_3d": { "position": [4.0, 2.0, 0.0], "rotation": [0.0, 0.0, 0.0, 1.0], "scale": [1.0, 1.0, 1.0] },
      "prefab": {
        "source": "prefabs/coin.prefab",
        "overrides": { "sparkle": { "components": { "sindri.sprite": { "layer": 5 } } } }
      }
    }
  ]
}"#;
    let document = SceneDocument::from_json(json).unwrap();
    let placed = document.entity(&id("coin-1")).unwrap();
    assert_eq!(
        placed.prefab,
        Some(PrefabInstance {
            source: "prefabs/coin.prefab".to_owned(),
            overrides: [(
                id("sparkle"),
                EntityOverride {
                    components: [("sindri.sprite".to_owned(), json!({ "layer": 5 }))].into(),
                    ..EntityOverride::default()
                }
            )]
            .into(),
        })
    );
    let written = document.to_canonical_json().unwrap();
    assert_eq!(SceneDocument::from_json(&written).unwrap(), document);
}
