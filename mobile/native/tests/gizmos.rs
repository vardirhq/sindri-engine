//! Non-rendered objects remain visible and movable through editor-only gizmos.
use std::path::Path;
use sindri_core::{EntityData, Transform3D};
use sindri_mobile::{Editing, OpenedScene};

const SIZE: [f32; 2] = [1080.0, 2400.0];

#[test]
fn a_sprite_less_object_can_be_selected_and_moved_by_its_gizmo() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let mut opened = OpenedScene::open(&project, "world.scene").unwrap();
    let object = opened.world.spawn(EntityData { name: Some("Marker".to_owned()), transform_3d: Some(Transform3D::default()), ..EntityData::default() });
    let mut editing = Editing::for_scene(&opened).unwrap();
    editing.canvas.center = [0.0, 0.0];
    let id = object.to_bits().to_string();
    assert!(editing.gizmos(&opened.world, SIZE).iter().any(|gizmo| gizmo["id"].as_str() == Some(id.as_str())));
    let per_pixel = editing.canvas.per_pixel(SIZE[1]);
    editing.press_gizmo(&opened.world, &id, [540.0, 1200.0]).unwrap();
    assert_eq!(editing.selected, Some(object));
    editing.drag(&mut opened.world, [640.0, 1200.0], SIZE).unwrap();
    editing.release();
    let position = opened.world.get(object).unwrap().transform_3d.unwrap().position;
    assert!((position[0] - 100.0 * per_pixel).abs() < 1e-3);
    editing.undo(&mut opened.world).unwrap();
    assert!(opened.world.get(object).unwrap().transform_3d.unwrap().position[0].abs() < 1e-3);
}

#[test]
fn sprite_and_screen_ui_objects_do_not_get_world_gizmos() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let mut opened = OpenedScene::open(&project, "world.scene").unwrap();
    let ui = opened.world.spawn(EntityData { transform_3d: Some(Transform3D::default()), components: [("sindri.ui.text".to_owned(), serde_json::json!({}))].into(), ..EntityData::default() });
    let sprite = opened.world.spawn(EntityData { transform_3d: Some(Transform3D::default()), components: [("sindri.sprite".to_owned(), serde_json::json!({}))].into(), ..EntityData::default() });
    let mut editing = Editing::for_scene(&opened).unwrap();
    editing.canvas.center = [0.0, 0.0];
    let id = ui.to_bits().to_string();
    assert!(!editing.gizmos(&opened.world, SIZE).iter().any(|gizmo| gizmo["id"].as_str() == Some(id.as_str())));
    assert!(editing.press_gizmo(&opened.world, &id, [0.0, 0.0]).is_err());
    let sprite_id = sprite.to_bits().to_string();
    assert!(!editing.gizmos(&opened.world, SIZE).iter().any(|gizmo| gizmo["id"].as_str() == Some(sprite_id.as_str())));
}

#[test]
fn moving_a_child_marker_uses_world_space_under_a_rotated_parent() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let mut opened = OpenedScene::open(&project, "world.scene").unwrap();
    let parent = opened.world.spawn(EntityData { transform_3d: Some(Transform3D { rotation: glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2).to_array(), scale: [2.0, 2.0, 1.0], ..Transform3D::default() }), ..EntityData::default() });
    let child = opened.world.spawn(EntityData { transform_3d: Some(Transform3D::default()), ..EntityData::default() });
    opened.world.set_parent(child, Some(parent)).unwrap();
    let mut editing = Editing::for_scene(&opened).unwrap();
    let before = opened.world.world_transform(child).unwrap().position;
    let per_pixel = editing.canvas.per_pixel(SIZE[1]);
    editing.press_gizmo(&opened.world, &child.to_bits().to_string(), [540.0, 1200.0]).unwrap();
    editing.drag(&mut opened.world, [640.0, 1200.0], SIZE).unwrap();
    editing.release();
    let after = opened.world.world_transform(child).unwrap().position;
    assert!((after[0] - before[0] - 100.0 * per_pixel).abs() < 1e-3);
    assert!((after[1] - before[1]).abs() < 1e-3);
}
