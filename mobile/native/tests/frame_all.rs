use std::path::Path;

use serde_json::json;
use sindri_core::{EntityData, Transform3D, World};
use sindri_mobile::{Editing, OpenedScene};

#[test]
fn frame_all_fits_tilemap_and_ignores_screen_ui() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let mut opened = OpenedScene::open(&project, "village.scene").unwrap();
    let mut editing = Editing::for_scene(&opened).unwrap();
    opened.world = World::default();
    opened.world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [("sindri.tilemap".to_owned(), json!({
            "texture": "textures/village-ground.png", "palette": ["tile"],
            "columns": 4, "rows": 2, "tile_size": [2.0, 3.0], "tiles": [0, 0, 0, 0, 0, 0, 0, 0]
        }))].into(),
        ..EntityData::default()
    });
    opened.world.spawn(EntityData {
        transform_3d: Some(Transform3D { position: [1000.0, 1000.0, 0.0], ..Transform3D::default() }),
        components: [("sindri.ui.text".to_owned(), json!({}))].into(),
        ..EntityData::default()
    });
    editing.frame_all(&opened, [600.0, 1200.0]).unwrap();
    assert!((editing.canvas.center[0] - 4.0).abs() < 1e-3);
    assert!((editing.canvas.center[1] + 3.0).abs() < 1e-3);
    assert!((editing.canvas.half_height - 9.6).abs() < 1e-3);
    assert!(!editing.unsaved());
}

#[test]
fn frame_all_leaves_an_empty_scene_alone() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let mut opened = OpenedScene::open(&project, "world.scene").unwrap();
    let mut editing = Editing::for_scene(&opened).unwrap();
    opened.world = World::default();
    let before = editing.canvas;
    editing.frame_all(&opened, [1080.0, 2400.0]).unwrap();
    assert_eq!(editing.canvas, before);
}
