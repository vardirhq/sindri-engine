//! Real padded village art: selection and handles follow the visible pixels.

use std::path::Path;

use sindri_core::SceneEntityId;
use sindri_mobile::{Editing, OpenedScene};

const SIZE: [f32; 2] = [1080.0, 2400.0];

#[test]
fn transparent_padding_pans_and_visible_lamp_picks_and_moves() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let mut scene = OpenedScene::open(&project, "village.scene").unwrap();
    let lamp = scene
        .world
        .entity_for_source_id(&SceneEntityId::new("lantern-4".to_owned()).unwrap())
        .unwrap();
    let start = scene
        .world
        .get(lamp)
        .unwrap()
        .transform_3d
        .unwrap()
        .position;
    let mut editing = Editing::for_scene(&scene).unwrap();
    editing.canvas.center = [start[0], start[1]];
    editing.selected = Some(lamp);
    let origin = editing.selection_pixel(&scene.world, SIZE).unwrap();
    let unit = editing.canvas.per_pixel(SIZE[1]);
    // The lamp is 15x45 pixels inside a 112x112 cell scaled to seven units.
    let visible = [origin[0], origin[1] - (7.0 * (1.0 - 75.5 / 112.0)) / unit];
    let empty = [origin[0] + 2.0 / unit, origin[1] - 3.5 / unit];
    assert_eq!(
        editing
            .pick(&scene.world, &scene.extractor, visible, SIZE)
            .unwrap(),
        Some(lamp)
    );
    assert_ne!(
        editing
            .pick(&scene.world, &scene.extractor, empty, SIZE)
            .unwrap(),
        Some(lamp)
    );
    let [left, top, right, bottom] = editing
        .selection_bounds(&scene.world, &scene.extractor, &scene.assets.sheets, SIZE)
        .unwrap()
        .unwrap();
    assert!(((right - left) * unit - 7.0 * 15.0 / 112.0).abs() < 0.03);
    assert!(((bottom - top) * unit - 7.0 * 45.0 / 112.0).abs() < 0.03);
    let center = editing.canvas.center;
    editing
        .press(&scene.world, &scene.extractor, empty, SIZE)
        .unwrap();
    editing
        .drag(&mut scene.world, [empty[0] + 40.0, empty[1]], SIZE)
        .unwrap();
    editing.release();
    assert!(
        (editing.canvas.center[0] - center[0]).abs() > 0.001,
        "empty padding pans"
    );
    let unchanged = scene
        .world
        .get(lamp)
        .unwrap()
        .transform_3d
        .unwrap()
        .position;
    assert!(
        unchanged
            .iter()
            .zip(start)
            .all(|(a, b)| (*a - b).abs() < 1.0e-5)
    );
    // Pan moved the lamp on screen; return the camera and drag the visible part.
    editing.canvas.center = [start[0], start[1]];
    editing
        .press(&scene.world, &scene.extractor, visible, SIZE)
        .unwrap();
    editing
        .drag(&mut scene.world, [visible[0] + 40.0, visible[1]], SIZE)
        .unwrap();
    editing.release();
    assert!(
        scene
            .world
            .get(lamp)
            .unwrap()
            .transform_3d
            .unwrap()
            .position[0]
            > start[0]
    );
}
