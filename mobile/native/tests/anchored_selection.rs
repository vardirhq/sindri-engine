//! The selection's box, and the handles placed on it, where a sprite is
//! drawn: a sheet that stands its sprites on their feet moves the picture
//! above the entity's origin, and the box must move with it.

use std::path::{Path, PathBuf};

use sindri_core::SceneEntityId;
use sindri_mobile::{Editing, OpenedScene};

const SIZE: [f32; 2] = [1080.0, 2400.0];

#[test]
fn a_sprite_standing_on_its_feet_is_boxed_where_it_is_drawn() {
    // The lantern's sheet anchors it at the bottom: the picture stands on its
    // origin, so the box (and the handles on it) must too, not straddle it.
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let opened = OpenedScene::open(&root, "world.scene").expect("the field opens");
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    let lantern = opened
        .world
        .entity_for_source_id(&SceneEntityId::new("lantern-1".to_owned()).expect("a valid ID"))
        .expect("the field has lantern-1");
    let at = opened
        .world
        .get(lantern)
        .and_then(|data| data.transform_3d)
        .expect("a transform")
        .position;
    editing.canvas.center = [at[0], at[1]];
    editing.selected = Some(lantern);
    let [left, top, right, bottom] = editing
        .selection_bounds(
            &opened.world,
            &opened.extractor,
            &opened.assets.sheets,
            SIZE,
        )
        .expect("bounds resolve")
        .expect("the lantern is on the canvas");
    let origin = editing
        .selection_pixel(&opened.world, SIZE)
        .expect("the lantern is on the canvas");
    assert!(
        (bottom - origin[1]).abs() < 1.0,
        "its feet on the origin: {bottom} vs {origin:?}"
    );
    assert!(
        top < origin[1] && left < origin[0] && origin[0] < right,
        "{left} {top} {right} {bottom}"
    );
}
