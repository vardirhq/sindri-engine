//! Selection verbs keep hierarchies and their undo history intact.

use std::path::Path;

use sindri_core::{EntityData, SceneEntityId};
use sindri_mobile::{Editing, OpenedScene};

fn scene() -> OpenedScene {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    OpenedScene::open(&project, "world.scene").expect("scene opens")
}

fn id(name: &str) -> SceneEntityId {
    SceneEntityId::new(name.to_owned()).expect("valid ID")
}

#[test]
fn duplicating_parent_and_child_copies_one_subtree_and_undoes_as_one() {
    let mut opened = scene();
    let parent = opened.world.spawn(EntityData {
        source_id: Some(id("test-parent")),
        ..Default::default()
    });
    let child = opened.world.spawn(EntityData {
        source_id: Some(id("test-child")),
        ..Default::default()
    });
    opened
        .world
        .set_parent(child, Some(parent))
        .expect("parenting");
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing");
    editing.selected = Some(parent);
    editing.toggle_selected(child);
    editing
        .duplicate_selection(&mut opened.world)
        .expect("duplicate");
    let copy = opened
        .world
        .entity_for_source_id(&id("test-parent-copy"))
        .expect("parent copied");
    let child_copy = opened
        .world
        .entity_for_source_id(&id("test-child-copy"))
        .expect("child copied");
    assert_eq!(opened.world.get(child_copy).unwrap().parent, Some(copy));
    assert_eq!(editing.selected_entities(), vec![copy]);
    editing.undo(&mut opened.world).expect("undo");
    assert!(opened.world.get(copy).is_none());
    assert!(opened.world.get(child_copy).is_none());
    assert!(editing.selected_entities().is_empty());
    editing.redo(&mut opened.world).expect("redo");
    assert!(opened.world.get(copy).is_some());
    assert_eq!(opened.world.get(child_copy).unwrap().parent, Some(copy));
}

#[test]
fn deleting_overlapping_selection_restores_the_subtree_on_undo() {
    let mut opened = scene();
    let parent = opened.world.spawn(EntityData {
        source_id: Some(id("delete-parent")),
        ..Default::default()
    });
    let child = opened.world.spawn(EntityData {
        source_id: Some(id("delete-child")),
        ..Default::default()
    });
    opened
        .world
        .set_parent(child, Some(parent))
        .expect("parenting");
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing");
    editing.selected = Some(parent);
    editing.toggle_selected(child);
    editing.delete_selection(&mut opened.world).expect("delete");
    assert!(opened.world.get(parent).is_none());
    assert!(opened.world.get(child).is_none());
    editing.undo(&mut opened.world).expect("undo");
    assert_eq!(opened.world.get(child).unwrap().parent, Some(parent));
    editing.redo(&mut opened.world).expect("redo");
    assert!(opened.world.get(parent).is_none());
}

#[test]
fn scene_objects_include_unnamed_prefab_children_and_select_by_runtime_handle() {
    let mut opened = scene();
    let child = opened.world.spawn(EntityData {
        name: Some("Screen HUD".to_owned()),
        ..Default::default()
    });
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing");
    let objects = editing.scene_objects(&opened.world);
    let handle = child.to_bits().to_string();
    assert!(
        objects
            .iter()
            .any(|row| row["id"].as_str() == Some(handle.as_str()) && row["name"] == "Screen HUD")
    );
    editing
        .select_object(&opened.world, &handle)
        .expect("select");
    assert_eq!(editing.selected, Some(child));
    assert!(editing.select_object(&opened.world, "not an id").is_err());
    assert_eq!(editing.selected, Some(child));
}

#[test]
fn hierarchy_reparent_preserves_world_position_and_undoes() {
    let mut opened = scene();
    let parent = opened.world.spawn(EntityData {
        name: Some("Test parent".to_owned()),
        transform_3d: Some(sindri_core::Transform3D {
            position: [10.0, 3.0, 0.0],
            scale: [2.0, 2.0, 1.0],
            ..Default::default()
        }),
        ..Default::default()
    });
    let child = opened.world.spawn(EntityData {
        name: Some("Test child".to_owned()),
        transform_3d: Some(sindri_core::Transform3D {
            position: [4.0, 7.0, 0.0],
            ..Default::default()
        }),
        ..Default::default()
    });
    let mut editing = Editing::new(&opened.world, &opened.extractor).unwrap();
    let parent_id = parent.to_bits().to_string();
    let child_id = child.to_bits().to_string();
    editing
        .reparent_object(&mut opened.world, &child_id, Some(&parent_id))
        .unwrap();
    let rows = editing.scene_objects(&opened.world);
    let parent_index = rows
        .iter()
        .position(|row| row["id"].as_str() == Some(parent_id.as_str()))
        .unwrap();
    assert_eq!(
        rows[parent_index + 1]["id"].as_str(),
        Some(child_id.as_str())
    );
    assert_eq!(rows[parent_index + 1]["depth"].as_u64(), Some(1));
    let position = opened.world.world_transform(child).unwrap().position;
    assert!((position[0] - 4.0).abs() < 1e-4 && (position[1] - 7.0).abs() < 1e-4);
    assert!(
        editing
            .reparent_object(&mut opened.world, &parent_id, Some(&child_id))
            .is_err()
    );
    editing.undo(&mut opened.world).unwrap();
    assert_eq!(opened.world.get(child).unwrap().parent, None);
    let position = opened.world.world_transform(child).unwrap().position;
    assert!((position[0] - 4.0).abs() < 1e-4 && (position[1] - 7.0).abs() < 1e-4);
    editing.redo(&mut opened.world).unwrap();
    assert_eq!(opened.world.get(child).unwrap().parent, Some(parent));
    editing
        .reparent_object(&mut opened.world, &child_id, None)
        .unwrap();
    assert_eq!(opened.world.get(child).unwrap().parent, None);
}
