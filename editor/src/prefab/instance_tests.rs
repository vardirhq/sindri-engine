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
    let instance = sindri_core::PrefabInstance {
        overrides,
        ..sindri_core::PrefabInstance::new(COIN)
    };
    let updated = applied(&prefab, &instance, &library(prefab.clone()));
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
    let instance = sindri_core::PrefabInstance {
        overrides,
        removed: [id("loot/sparkle")].into(),
        ..sindri_core::PrefabInstance::new("prefabs/chest.prefab")
    };
    let updated = applied(&chest, &instance, &library(coin("gold.png")));
    let nested = updated.entities[1].prefab.as_ref().unwrap();
    assert_eq!(
        nested.overrides[&id("coin")].components["sindri.sprite"],
        json!({ "layer": 5 })
    );
    assert_eq!(nested.overrides[&id("sparkle")].disabled, Some(true));
    assert!(
        nested.removed.contains(&id("sparkle")),
        "removed inside the chest's coin"
    );
}

#[test]
fn applying_a_removal_takes_the_entity_out_of_the_prefab() {
    let prefab = coin("gold.png");
    let instance = sindri_core::PrefabInstance {
        removed: [id("sparkle")].into(),
        ..sindri_core::PrefabInstance::new(COIN)
    };
    let updated = applied(&prefab, &instance, &library(prefab.clone()));
    assert_eq!(updated.entities.len(), 1);
}

#[test]
fn deleting_inside_an_instance_is_undone_by_restoring_it() {
    let prefabs = library(coin("gold.png"));
    let mut world = World::default();
    let mut history = CommandHistory::default();
    let root = placed(&mut world, &mut history, &prefabs);
    let sparkle = world.entity_for_source_id(&id("coin/sparkle")).unwrap();
    let mut buffer = CommandBuffer::new();
    buffer.push(sindri_core::WorldCommand::Despawn { entity: sparkle });
    apply(&mut world, &mut history, buffer);

    let mut reference = world.instance_entity(root, &prefabs).unwrap();
    let instance = reference.prefab.as_mut().unwrap();
    assert!(instance.removed.contains(&id("sparkle")));
    instance.removed.clear();
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
    assert!(world.entity_for_source_id(&id("coin/sparkle")).is_some());
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
            transform_3d: Some(Transform3D {
                position: [2.0, 0.0, 0.0],
                ..Transform3D::default()
            }),
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

/// A prefab opened to be edited resolves what it names against the scene's
/// folder, not its own, and saves back as a prefab.
#[test]
fn a_prefab_opens_as_a_document_and_saves_as_a_prefab() {
    let directory = std::env::temp_dir().join(format!("sindri-prefab-doc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("prefabs")).unwrap();
    std::fs::write(
        directory.join("level.scene"),
        SceneDocument::default().to_canonical_json().unwrap(),
    )
    .unwrap();
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
    let path = directory.join("prefabs/chest.prefab");
    std::fs::write(&path, chest.to_canonical_json().unwrap()).unwrap();
    std::fs::write(
        directory.join("prefabs/coin.prefab"),
        coin("gold.png").to_canonical_json().unwrap(),
    )
    .unwrap();

    let mut file = crate::scene_file::SceneFile::open(&path).unwrap();
    assert!(file.is_prefab());
    assert_eq!(
        file.anchor().and_then(std::path::Path::parent),
        Some(directory.as_path())
    );
    let world = World::from_scene_with(file.document(), file.prefabs())
        .unwrap()
        .world;
    assert_eq!(world.len(), 3, "the chest and the coin in it");
    file.save(&world).unwrap();
    let saved = PrefabDocument::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(saved, chest.canonicalized());

    let mut two_roots = world.clone();
    two_roots.spawn(sindri_core::EntityData {
        source_id: Some(id("stray")),
        ..sindri_core::EntityData::default()
    });
    assert!(file.save(&two_roots).is_err(), "a prefab keeps one root");
    let _ = std::fs::remove_dir_all(&directory);
}

/// A subtree becomes a prefab with its root at the origin, and an instance
/// inside it stays one.
#[test]
fn a_subtree_becomes_a_prefab_that_nests_the_instances_in_it() {
    let prefabs = library(coin("gold.png"));
    let scene = SceneDocument {
        entities: vec![
            SceneEntity {
                name: Some("Chest".to_owned()),
                transform_3d: Some(Transform3D {
                    position: [5.0, 1.0, 2.0],
                    ..Transform3D::default()
                }),
                ..SceneEntity::new(id("chest"))
            },
            SceneEntity {
                parent: Some(id("chest")),
                ..SceneEntity::instance(id("loot"), COIN)
            },
            SceneEntity::new(id("elsewhere")),
        ],
        ..SceneDocument::default()
    };
    let world = World::from_scene_with(&scene, &prefabs).unwrap().world;
    let chest = world.entity_for_source_id(&id("chest")).unwrap();
    let prefab = super::subtree_prefab(&world, chest, &prefabs).unwrap();
    let ids: Vec<&str> = prefab
        .entities
        .iter()
        .map(|entity| entity.id.as_str())
        .collect();
    assert_eq!(ids, ["chest", "loot"]);
    let position = prefab.entities[0].transform_3d.unwrap().position;
    assert!(
        position
            .iter()
            .zip([0.0, 0.0, 2.0])
            .all(|(found, wanted)| (found - wanted).abs() < f32::EPSILON),
        "at the origin, at its own depth: {position:?}"
    );
    assert!(
        prefab.entities[1].prefab.is_some(),
        "the coin is still an instance"
    );
}

/// A scene whose prefab has gone opens with a placeholder for the instance,
/// and saves the instance back exactly as it was written.
#[test]
fn a_scene_with_a_missing_prefab_opens_and_saves_it_back_unchanged() {
    let directory = std::env::temp_dir().join(format!("sindri-missing-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let mut changes = EntityOverride::default();
    changes
        .components
        .insert("sindri.sprite".to_owned(), json!({ "layer": 9 }));
    let placed = SceneEntity {
        name: Some("Lucky".to_owned()),
        transform_3d: Some(Transform3D {
            position: [2.0, 0.0, 0.0],
            ..Transform3D::default()
        }),
        prefab: Some(sindri_core::PrefabInstance {
            overrides: BTreeMap::from([(id("coin"), changes)]),
            ..sindri_core::PrefabInstance::new(COIN)
        }),
        ..SceneEntity::new(id("coin-1"))
    };
    let glint = SceneEntity {
        parent: Some(id("coin-1/sparkle")),
        ..SceneEntity::new(id("glint"))
    };
    let scene = SceneDocument {
        entities: vec![placed, glint],
        ..SceneDocument::default()
    }
    .canonicalized();
    let path = directory.join("level.scene");
    crate::scene_file::SceneFile::create(&path, &scene).unwrap();

    let mut file = crate::scene_file::SceneFile::open(&path).expect("it opens");
    assert_eq!(file.missing().len(), 1);
    let world = crate::native::load_world(&crate::native::scene_extractor(), &file).unwrap();
    assert_eq!(world.len(), 2, "a placeholder, and the entity under it");
    file.save(&world).unwrap();
    let saved =
        SceneDocument::from_json(&std::fs::read_to_string(&path).unwrap()).expect("it reads");
    assert_eq!(saved, scene);
    let _ = std::fs::remove_dir_all(&directory);
}
