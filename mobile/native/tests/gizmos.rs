//! Markers for objects with nothing drawn: only where nothing else can be
//! tapped, picked after the drawn art, and moved only once selected.
use std::path::Path;

use serde_json::json;
use sindri_core::{EntityData, EntityId, SceneEntityId, Transform3D};
use sindri_mobile::{Editing, OpenedScene};

const SIZE: [f32; 2] = [1080.0, 2400.0];

fn lamplighter(scene: &str) -> OpenedScene {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    OpenedScene::open(&project, scene).unwrap()
}

fn entity(opened: &OpenedScene, id: &str) -> EntityId {
    opened
        .world
        .entity_for_source_id(&SceneEntityId::new(id.to_owned()).unwrap())
        .unwrap()
}

fn marker_at(editing: &Editing, opened: &OpenedScene, entity: EntityId) -> Option<[f32; 2]> {
    let id = entity.to_bits().to_string();
    editing
        .gizmos(&opened.world, SIZE)
        .into_iter()
        .find(|gizmo| gizmo["id"].as_str() == Some(id.as_str()))
        .map(|gizmo| {
            #[allow(clippy::cast_possible_truncation)] // Screen pixels fit in f32.
            let read = |key: &str| gizmo[key].as_f64().unwrap() as f32;
            [read("x"), read("y")]
        })
}

fn spawn_at(opened: &mut OpenedScene, position: [f32; 3]) -> EntityId {
    opened.world.spawn(EntityData {
        name: Some("Marker".to_owned()),
        transform_3d: Some(Transform3D {
            position,
            ..Transform3D::default()
        }),
        ..EntityData::default()
    })
}

#[test]
fn the_field_marks_only_what_cannot_otherwise_be_tapped() {
    let opened = lamplighter("world.scene");
    let editing = Editing::for_scene(&opened).unwrap();
    let marked = editing.marked(&opened.world);
    // Settings objects have no place in the world.
    for settings in ["environment", "physics"] {
        assert!(!marked.contains(&entity(&opened, settings)), "{settings}");
    }
    // Wick's lantern, its lights and its particles are parts of a drawn
    // object, reached through it or Layers.
    for part in ["wick", "wick-lantern", "wick-light", "wick-halo"] {
        assert!(!marked.contains(&entity(&opened, part)), "{part}");
    }
    // The lamp lights stand alone, so they keep theirs.
    assert!(marked.contains(&entity(&opened, "lantern-1-light")));
    assert!(marked.len() < 20, "{} markers", marked.len());
}

#[test]
fn a_selected_part_shows_its_marker_and_moves_by_it() {
    let mut opened = lamplighter("world.scene");
    let light = entity(&opened, "wick-light");
    let mut editing = Editing::for_scene(&opened).unwrap();
    assert!(marker_at(&editing, &opened, light).is_none());
    editing.selected = Some(light);
    let at = marker_at(&editing, &opened, light).expect("the selection's marker");
    let before = opened.world.world_transform(light).unwrap().position;
    let per_pixel = editing.canvas.per_pixel(SIZE[1]);
    editing
        .press(&opened.world, &opened.extractor, at, SIZE)
        .unwrap();
    editing
        .drag(&mut opened.world, [at[0] + 100.0, at[1]], SIZE)
        .unwrap();
    editing.release();
    let after = opened.world.world_transform(light).unwrap().position;
    assert!((after[0] - before[0] - 100.0 * per_pixel).abs() < 1e-3);
    assert!((after[1] - before[1]).abs() < 1e-3);
}

#[test]
fn a_marker_is_tapped_to_select_then_dragged_to_move() {
    let mut opened = lamplighter("world.scene");
    let mut editing = Editing::for_scene(&opened).unwrap();
    let center = editing.canvas.center;
    let object = spawn_at(&mut opened, [center[0] + 3.0, center[1] + 3.0, 0.0]);
    let at = marker_at(&editing, &opened, object).expect("a root object's marker");

    // A drag that starts on a marker not yet selected pans the view.
    editing
        .press(&opened.world, &opened.extractor, at, SIZE)
        .unwrap();
    editing
        .drag(&mut opened.world, [at[0] + 100.0, at[1]], SIZE)
        .unwrap();
    editing.release();
    let still = opened
        .world
        .get(object)
        .unwrap()
        .transform_3d
        .unwrap()
        .position;
    assert!(
        (still[0] - center[0] - 3.0).abs() < 1e-6,
        "the marker did not move"
    );
    assert!((editing.canvas.center[0] - center[0]).abs() > 1e-3);

    // A tap selects it; then a drag moves it, as one undo step.
    let at = marker_at(&editing, &opened, object).unwrap();
    editing
        .tap(
            &opened.world,
            &opened.extractor,
            [at[0] + 10.0, at[1]],
            SIZE,
        )
        .unwrap();
    assert_eq!(editing.selected, Some(object));
    let per_pixel = editing.canvas.per_pixel(SIZE[1]);
    editing
        .press(&opened.world, &opened.extractor, at, SIZE)
        .unwrap();
    editing
        .drag(&mut opened.world, [at[0] + 100.0, at[1]], SIZE)
        .unwrap();
    editing.release();
    let moved = opened
        .world
        .get(object)
        .unwrap()
        .transform_3d
        .unwrap()
        .position;
    assert!((moved[0] - center[0] - 3.0 - 100.0 * per_pixel).abs() < 1e-3);
    editing.undo(&mut opened.world).unwrap();
    let back = opened
        .world
        .get(object)
        .unwrap()
        .transform_3d
        .unwrap()
        .position;
    assert!((back[0] - center[0] - 3.0).abs() < 1e-3);
}

#[test]
fn tapping_again_steps_from_the_art_to_the_marker_under_it() {
    let mut opened = lamplighter("village.scene");
    let lamp = entity(&opened, "lantern-4");
    let start = opened
        .world
        .get(lamp)
        .unwrap()
        .transform_3d
        .unwrap()
        .position;
    // The lamp's visible post is this far above its origin (see sprite_picking).
    let rise = 7.0 * (1.0 - 75.5 / 112.0);
    let light = spawn_at(&mut opened, [start[0], start[1] + rise, start[2]]);
    opened
        .world
        .get_mut(light)
        .unwrap()
        .components
        .insert("sindri.light2d".to_owned(), json!({}));
    let mut editing = Editing::for_scene(&opened).unwrap();
    editing.canvas.center = [start[0], start[1]];
    let on = marker_at(&editing, &opened, light).expect("a standalone light's marker");
    let tap = |editing: &mut Editing| {
        editing
            .tap(&opened.world, &opened.extractor, on, SIZE)
            .unwrap();
        editing.selected
    };
    assert_eq!(tap(&mut editing), Some(lamp), "the art comes first");
    assert_eq!(tap(&mut editing), Some(light), "then what lies under it");
    // The village has markers of its own nearby; tapping on goes through
    // them and comes back round to the lamp.
    assert!(
        (0..8).any(|_| tap(&mut editing) == Some(lamp)),
        "tapping on comes back round"
    );
}

#[test]
fn sprite_and_screen_ui_objects_do_not_get_markers() {
    let mut opened = lamplighter("world.scene");
    let ui = opened.world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [("sindri.ui.text".to_owned(), json!({}))].into(),
        ..EntityData::default()
    });
    let sprite = opened.world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [("sindri.sprite".to_owned(), json!({}))].into(),
        ..EntityData::default()
    });
    let mut editing = Editing::for_scene(&opened).unwrap();
    editing.canvas.center = [0.0, 0.0];
    for entity in [ui, sprite] {
        editing.selected = Some(entity);
        assert!(marker_at(&editing, &opened, entity).is_none());
    }
}

#[test]
fn moving_a_child_marker_uses_world_space_under_a_rotated_parent() {
    let mut opened = lamplighter("world.scene");
    let parent = opened.world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            rotation: glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2).to_array(),
            scale: [2.0, 2.0, 1.0],
            ..Transform3D::default()
        }),
        ..EntityData::default()
    });
    let child = opened.world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        ..EntityData::default()
    });
    opened.world.set_parent(child, Some(parent)).unwrap();
    let mut editing = Editing::for_scene(&opened).unwrap();
    editing.canvas.center = [0.0, 0.0];
    editing.selected = Some(child);
    let at = marker_at(&editing, &opened, child).unwrap();
    let before = opened.world.world_transform(child).unwrap().position;
    let per_pixel = editing.canvas.per_pixel(SIZE[1]);
    editing
        .press(&opened.world, &opened.extractor, at, SIZE)
        .unwrap();
    editing
        .drag(&mut opened.world, [at[0] + 100.0, at[1]], SIZE)
        .unwrap();
    editing.release();
    let after = opened.world.world_transform(child).unwrap().position;
    assert!((after[0] - before[0] - 100.0 * per_pixel).abs() < 1e-3);
    assert!((after[1] - before[1]).abs() < 1e-3);
}
