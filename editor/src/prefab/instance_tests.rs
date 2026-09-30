//! Instances in the editor: made, brought up to date, reverted, applied, and
//! written back as references.

use std::collections::BTreeMap;

use serde_json::json;
use sindri_core::{
    CommandBuffer, CommandHistory, EntityOverride, PrefabDocument, SceneDocument, SceneEntity,
    SceneEntityId, Transform3D, World,
};

use super::{applied, reconcile, spawn_instance};

const COIN: &str = "prefabs/coin.prefab";

fn id(value: &str) -> SceneEntityId {
    SceneEntityId::new(value).unwrap()
}

fn coin(texture: &str) -> PrefabDocument {
    let mut root = SceneEntity {
        name: Some("Coin".to_owned()),
        transform_3d: Some(Transform3D::default()),
        ..SceneEntity::new(id("coin"))
    };
    root.components.insert(
        "sindri.sprite".to_owned(),
        json!({ "texture": texture, "layer": 2 }),
    );
    let sparkle = SceneEntity {
        name: Some("Sparkle".to_owned()),
        parent: Some(id("coin")),
        ..SceneEntity::new(id("sparkle"))
    };
    PrefabDocument {
        entities: vec![root, sparkle],
        ..PrefabDocument::default()
    }
}

fn library(prefab: PrefabDocument) -> BTreeMap<String, PrefabDocument> {
    BTreeMap::from([(COIN.to_owned(), prefab)])
}

fn apply(world: &mut World, history: &mut CommandHistory, buffer: CommandBuffer) {
    history
        .apply(buffer.into_transaction("test"), world)
        .expect("the commands apply");
}

fn sprite(world: &World, source_id: &str) -> serde_json::Value {
    let entity = world.entity_for_source_id(&id(source_id)).unwrap();
    world.get(entity).unwrap().components["sindri.sprite"].clone()
}

/// Places one coin and answers with its root.
fn placed(
    world: &mut World,
    history: &mut CommandHistory,
    prefabs: &BTreeMap<String, PrefabDocument>,
) -> sindri_core::EntityId {
    let mut rehearsal = world.clone();
    let mut buffer = CommandBuffer::new();
    let root = spawn_instance(&mut rehearsal, COIN, prefabs, None, None, &mut buffer).unwrap();
    apply(world, history, buffer);
    root
}

#[test]
fn a_placed_instance_is_linked_and_saves_as_a_reference() {
    let prefabs = library(coin("gold.png"));
    let mut world = World::default();
    let mut history = CommandHistory::default();
    placed(&mut world, &mut history, &prefabs);
    placed(&mut world, &mut history, &prefabs);

    let saved = world.to_scene_with(&prefabs).unwrap();
    let ids: Vec<&str> = saved
        .entities
        .iter()
        .map(|entity| entity.id.as_str())
        .collect();
    assert_eq!(
        ids,
        ["coin", "coin-2"],
        "two instances, written as two entities"
    );
    assert!(saved.entities.iter().all(|entity| entity.prefab.is_some()));
    assert!(world.entity_for_source_id(&id("coin-2/sparkle")).is_some());
}

#[test]
fn a_changed_prefab_reaches_the_instance_as_one_undoable_step_keeping_overrides() {
    let before = library(coin("gold.png"));
    let mut world = World::default();
    let mut history = CommandHistory::default();
    let root = placed(&mut world, &mut history, &before);
    let sparkle = world.entity_for_source_id(&id("coin/sparkle")).unwrap();
    // An override made in the scene: the sparkle gains a component.
    world
        .get_mut(sparkle)
        .unwrap()
        .components
        .insert("sindri.tags".to_owned(), json!({ "tags": ["shiny"] }));

    let reference = world.instance_entity(root, &before).unwrap();
    let mut edited = coin("silver.png");
    edited.entities.push(SceneEntity {
        parent: Some(id("coin")),
        ..SceneEntity::new(id("shadow"))
    });
    let after = library(edited);
    let mut rehearsal = world.clone();
    let mut buffer = CommandBuffer::new();
    let members = world.instance_members(root);
    reconcile(
        &world,
        &mut rehearsal,
        &members,
        &reference,
        &after,
        &mut buffer,
    )
    .unwrap();
    apply(&mut world, &mut history, buffer);

    assert_eq!(sprite(&world, "coin")["texture"], "silver.png");
    assert!(world.entity_for_source_id(&id("coin/shadow")).is_some());
    assert_eq!(
        world.entity_for_source_id(&id("coin/sparkle")),
        Some(sparkle),
        "same handle"
    );
    assert!(
        world
            .get(sparkle)
            .unwrap()
            .components
            .contains_key("sindri.tags")
    );

    history.undo(&mut world).unwrap();
    assert_eq!(sprite(&world, "coin")["texture"], "gold.png");
    assert!(world.entity_for_source_id(&id("coin/shadow")).is_none());
}

#[test]
fn reverting_an_instance_takes_its_overrides_back() {
    let prefabs = library(coin("gold.png"));
    let mut world = World::default();
    let mut history = CommandHistory::default();
    let root = placed(&mut world, &mut history, &prefabs);
    world.get_mut(root).unwrap().components.insert(
        "sindri.sprite".to_owned(),
        json!({ "texture": "gold.png", "layer": 9 }),
    );

    let mut reference = world.instance_entity(root, &prefabs).unwrap();
    assert!(!reference.prefab.as_ref().unwrap().overrides.is_empty());
    reference.prefab.as_mut().unwrap().overrides.clear();
    let mut rehearsal = world.clone();
    let mut buffer = CommandBuffer::new();
    let members = world.instance_members(root);
    reconcile(
        &world,
        &mut rehearsal,
        &members,
        &reference,
        &prefabs,
        &mut buffer,
    )
    .unwrap();
    apply(&mut world, &mut history, buffer);
    assert_eq!(sprite(&world, "coin")["layer"], 2);
}

#[test]
fn applying_writes_an_instances_overrides_into_its_prefab() {
    let prefab = coin("gold.png");
    let mut changes = EntityOverride::default();
    changes
        .components
        .insert("sindri.sprite".to_owned(), json!({ "layer": 9 }));
    let overrides = BTreeMap::from([(id("coin"), changes)]);
    let updated = applied(&prefab, &overrides, &library(prefab.clone()));
    assert_eq!(
        updated.entities[0].components["sindri.sprite"],
        json!({ "texture": "gold.png", "layer": 9 })
    );
}

#[test]
fn applying_into_a_nested_instance_changes_the_outer_prefab_only() {
    let chest = PrefabDocument {
        entities: vec![
            SceneEntity::new(id("chest")),
            SceneEntity {
                parent: Some(id("chest")),
                ..SceneEntity::instance(id("loot"), COIN)
            },
        ],
        ..PrefabDocument::default()
    };
    let mut on_root = EntityOverride::default();
    on_root
        .components
        .insert("sindri.sprite".to_owned(), json!({ "layer": 5 }));
    let on_sparkle = EntityOverride {
        disabled: Some(true),
        ..EntityOverride::default()
    };
    let overrides = BTreeMap::from([(id("loot"), on_root), (id("loot/sparkle"), on_sparkle)]);
    let updated = applied(&chest, &overrides, &library(coin("gold.png")));
    let nested = updated.entities[1].prefab.as_ref().unwrap();
    assert_eq!(
        nested.overrides[&id("coin")].components["sindri.sprite"],
        json!({ "layer": 5 })
    );
    assert_eq!(nested.overrides[&id("sparkle")].disabled, Some(true));
}

#[test]
fn a_scene_file_opens_its_instances_and_saves_them_back() {
    let directory = std::env::temp_dir().join(format!("sindri-instances-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("prefabs")).unwrap();
    std::fs::write(
        directory.join("prefabs/coin.prefab"),
        coin("gold.png").to_canonical_json().unwrap(),
    )
    .unwrap();
    let scene = SceneDocument {
        entities: vec![SceneEntity {
            transform_3d: Some(Transform3D::default()),
            ..SceneEntity::instance(id("coin-1"), COIN)
        }],
        ..SceneDocument::default()
    };
    let path = directory.join("level.scene");
    crate::scene_file::SceneFile::create(&path, &scene).unwrap();

    let mut file = crate::scene_file::SceneFile::open(&path).unwrap();
    let world = World::from_scene_with(file.document(), file.prefabs())
        .unwrap()
        .world;
    assert_eq!(world.len(), 2);
    file.save(&world).unwrap();
    assert_eq!(file.document(), &scene.canonicalized());
    let _ = std::fs::remove_dir_all(&directory);
}
