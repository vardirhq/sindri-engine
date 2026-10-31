//! Slice step 2 without a phone: Lamplighter's field panned, pinched, tapped,
//! dragged and inspected through `Editing`, as the app's gestures drive it.
//!
//! Pixels are the Pixel 8's portrait screen. What a finger actually reports on
//! the device is the device check in `mobile/first-slice.md`.

use std::path::{Path, PathBuf};

use serde_json::json;
use sindri_core::{EntityId, SceneEntityId};
use sindri_mobile::{Editing, OpenedScene};

const SIZE: [f32; 2] = [1080.0, 2400.0];
const LIGHT: &str = "sindri.light2d";

fn opened() -> OpenedScene {
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    OpenedScene::open(&root, "world.scene").expect("the field opens")
}

fn entity(opened: &OpenedScene, id: &str) -> EntityId {
    opened
        .world
        .entity_for_source_id(&SceneEntityId::new(id.to_owned()).expect("a valid ID"))
        .unwrap_or_else(|| panic!("the field has {id}"))
}

fn position(opened: &OpenedScene, entity: EntityId) -> [f32; 3] {
    opened
        .world
        .get(entity)
        .and_then(|data| data.transform_3d)
        .expect("a transform")
        .position
}

/// Centers the canvas on `entity`, which is off the screen the field opens on.
fn look_at(editing: &mut Editing, opened: &OpenedScene, entity: EntityId) {
    let at = position(opened, entity);
    editing.canvas.center = [at[0], at[1]];
}

fn near(left: f32, right: f32) -> bool {
    (left - right).abs() < 1e-3
}

#[test]
fn the_canvas_opens_where_the_scene_camera_looks() {
    let opened = opened();
    let editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    let camera = opened
        .extractor
        .world_camera(&opened.world, sindri_scene::CameraView::default())
        .expect("the camera resolves")
        .expect("the field has a camera");
    assert!(near(editing.canvas.half_height, camera.framed_half_height));
    let eye = camera.view.inverse().w_axis;
    assert!(near(editing.canvas.center[0], eye.x) && near(editing.canvas.center[1], eye.y));
}

#[test]
fn tapping_where_the_canvas_draws_an_entity_picks_that_entity() {
    let opened = opened();
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    // Pan and pinch first, so the check covers a moved view, not only the
    // one the scene opened on.
    let lamp = entity(&opened, "cover-17");
    look_at(&mut editing, &opened, lamp);
    editing.canvas.pan([-140.0, 90.0], SIZE);
    editing.canvas.zoom(1.6, [300.0, 1500.0], SIZE);
    editing.selected = Some(lamp);
    let pixel = editing
        .selection_pixel(&opened.world, SIZE)
        .expect("the lamp is on the canvas");
    editing.selected = None;
    editing
        .tap(&opened.world, &opened.extractor, pixel, SIZE)
        .expect("the tap picks");
    let picked = editing
        .selected
        .expect("something is under the lamp's origin");
    let named = |entity: EntityId| {
        opened
            .world
            .get(entity)
            .and_then(|data| data.source_id.as_ref())
            .map(|id| id.as_str().to_owned())
    };
    assert_eq!(named(picked), named(lamp), "the canvas and the pick agree");
}

#[test]
fn a_drag_on_the_selection_moves_it_in_one_undo_step_and_elsewhere_pans() {
    let mut opened = opened();
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    let lamp = entity(&opened, "cover-17");
    let start = position(&opened, lamp);
    look_at(&mut editing, &opened, lamp);
    editing.selected = Some(lamp);
    let on = editing
        .selection_pixel(&opened.world, SIZE)
        .expect("the lamp is on the canvas");

    let per_pixel = editing.canvas.per_pixel(SIZE[1]);
    editing
        .press(&opened.world, &opened.extractor, on, SIZE)
        .expect("the press picks");
    for step in 1..=4_u8 {
        let travelled = f32::from(step) * 25.0;
        editing
            .drag(
                &mut opened.world,
                [on[0] + travelled, on[1] + travelled],
                SIZE,
            )
            .expect("the move applies");
    }
    editing.release();
    let moved = position(&opened, lamp);
    assert!(near(moved[0], start[0] + 100.0 * per_pixel), "{moved:?}");
    assert!(
        near(moved[1], start[1] - 100.0 * per_pixel),
        "down the screen is down the world"
    );
    assert!(near(moved[2], start[2]), "depth is kept");
    assert_eq!(
        editing.history.undo_steps().len(),
        1,
        "one drag, one undo step"
    );

    // A drag that starts off the selection moves the view, not the world.
    let canvas = editing.canvas;
    let revision = editing.history.revision();
    editing
        .press(&opened.world, &opened.extractor, [5.0, 5.0], SIZE)
        .expect("the press picks");
    editing
        .drag(&mut opened.world, [205.0, 5.0], SIZE)
        .expect("the pan applies");
    editing.release();
    assert!(near(
        editing.canvas.center[0],
        canvas.center[0] - 200.0 * per_pixel
    ));
    assert_eq!(
        editing.history.revision(),
        revision,
        "panning records nothing"
    );

    editing.undo(&mut opened.world).expect("undo applies");
    let back = position(&opened, lamp);
    assert!(
        near(back[0], start[0]) && near(back[1], start[1]),
        "{back:?}"
    );
    editing.redo(&mut opened.world).expect("redo applies");
    assert!(near(position(&opened, lamp)[0], moved[0]));
}

#[test]
fn pinching_keeps_the_point_under_the_fingers() {
    let opened = opened();
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    let focus = [200.0, 700.0];
    let before = editing.canvas.world_point(focus, SIZE);
    let half_height = editing.canvas.half_height;
    editing.canvas.zoom(2.0, focus, SIZE);
    let after = editing.canvas.world_point(focus, SIZE);
    assert!(near(before[0], after[0]) && near(before[1], after[1]));
    assert!(near(editing.canvas.half_height, half_height / 2.0));
}

#[test]
fn a_field_edit_is_checked_shown_and_undone() {
    let mut opened = opened();
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    let lamp = entity(&opened, "cover-17");
    editing.selected = Some(lamp);

    let refused = editing
        .set_field(
            &mut opened.world,
            &opened.extractor,
            LIGHT,
            "intensity",
            json!("bright"),
        )
        .expect_err("a light's intensity is a number");
    assert!(refused.contains("intensity"), "{refused}");
    assert_eq!(
        editing.history.undo_steps().len(),
        0,
        "a refusal leaves no step"
    );

    editing
        .set_field(
            &mut opened.world,
            &opened.extractor,
            LIGHT,
            "intensity",
            json!(1.5),
        )
        .expect("the edit applies");
    let shown = editing.inspector(&opened.world, &opened.extractor);
    let light = shown["components"]
        .as_array()
        .expect("components")
        .iter()
        .find(|component| component["type"] == LIGHT)
        .expect("the light is shown");
    let intensity = light["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .find(|field| field["name"] == "intensity")
        .expect("intensity is shown");
    assert_eq!(intensity["value"], json!(1.5));
    assert!(
        shown["undo"]
            .as_str()
            .is_some_and(|label| label.contains("intensity")),
        "{}",
        shown["undo"]
    );

    editing.undo(&mut opened.world).expect("undo applies");
    let restored = &opened.world.get(lamp).expect("the lamp").components[LIGHT]["intensity"];
    assert_eq!(restored, &json!(0.7));
}

#[test]
fn nothing_selected_shows_nothing_and_edits_nothing() {
    let mut opened = opened();
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    assert!(
        editing
            .inspector(&opened.world, &opened.extractor)
            .is_null()
    );
    assert!(
        editing
            .set_field(
                &mut opened.world,
                &opened.extractor,
                LIGHT,
                "intensity",
                json!(1.0)
            )
            .is_err()
    );
}
