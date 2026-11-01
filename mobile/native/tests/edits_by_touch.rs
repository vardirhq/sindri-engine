//! Slice step 2 without a phone: Lamplighter's field panned, pinched, tapped,
//! dragged, turned, scaled, outlined and inspected through `Editing`, as the
//! app's gestures drive it.
//!
//! Pixels are the Pixel 8's portrait screen. What a finger actually reports on
//! the device is the device check in `mobile/first-slice.md`.

use std::path::{Path, PathBuf};

use serde_json::json;
use sindri_core::{EntityId, SceneEntityId};
use sindri_mobile::{Editing, Handle, OpenedScene};

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
            false,
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
            false,
        )
        .expect("the edit applies");
    let shown = editing.inspector(&opened.world, &opened.extractor, &opened.assets.sheets);
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
            .inspector(&opened.world, &opened.extractor, &opened.assets.sheets)
            .is_null()
    );
    assert!(
        editing
            .set_field(
                &mut opened.world,
                &opened.extractor,
                LIGHT,
                "intensity",
                json!(1.0),
                false,
            )
            .is_err()
    );
}

#[test]
fn the_selection_outline_encloses_the_selection_and_follows_the_view() {
    let opened = opened();
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    let lamp = entity(&opened, "cover-17");
    look_at(&mut editing, &opened, lamp);
    assert_eq!(
        editing
            .selection_bounds(&opened.world, &opened.extractor, SIZE)
            .expect("bounds resolve"),
        None,
        "nothing selected, nothing outlined"
    );
    editing.selected = Some(lamp);
    let outline = |editing: &Editing| {
        editing
            .selection_bounds(&opened.world, &opened.extractor, SIZE)
            .expect("bounds resolve")
            .expect("the lamp is on the canvas")
    };
    let [left, top, right, bottom] = outline(&editing);
    let origin = editing
        .selection_pixel(&opened.world, SIZE)
        .expect("the lamp is on the canvas");
    assert!(
        left < right && top < bottom,
        "a real box: {left} {top} {right} {bottom}"
    );
    assert!(
        (left..=right).contains(&origin[0]) && (top..=bottom).contains(&origin[1]),
        "the box holds the lamp's origin {origin:?}"
    );
    // Panning moves the outline with the picture, pixel for pixel.
    editing.canvas.pan([40.0, -30.0], SIZE);
    let [moved_left, moved_top, ..] = outline(&editing);
    assert!(
        (moved_left - left - 40.0).abs() < 0.5,
        "{moved_left} vs {left}"
    );
    assert!((moved_top - top + 30.0).abs() < 0.5, "{moved_top} vs {top}");
}

fn transform(opened: &OpenedScene, entity: EntityId) -> sindri_core::Transform3D {
    opened
        .world
        .get(entity)
        .and_then(|data| data.transform_3d)
        .expect("a transform")
}

#[test]
fn the_canvas_draws_the_selection_outlined_and_the_scene_untouched() {
    let opened = opened();
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    let lamp = entity(&opened, "cover-17");
    look_at(&mut editing, &opened, lamp);
    let count = opened.world.entities().count();
    assert_eq!(
        editing.drawn(&opened.world, SIZE, 4.0).entities().count(),
        count,
        "nothing selected draws the scene itself"
    );

    editing.selected = Some(lamp);
    let drawn = editing.drawn(&opened.world, SIZE, 4.0);
    let edges: Vec<_> = drawn
        .entities()
        .filter(|(id, _)| opened.world.get(*id).is_none())
        .map(|(_, data)| data.clone())
        .collect();
    assert_eq!(edges.len(), 8, "an edge copy each way around the lamp");
    let lamp_at = position(&opened, lamp);
    for edge in &edges {
        let sprite = &edge.components["sindri.sprite"];
        assert_eq!(sprite["unlit"], json!(true), "the edge shows in the dark");
        assert_eq!(
            sprite["color_transform"]["multiply"],
            json!([0.0, 0.0, 0.0, 1.0]),
            "the edge is one flat colour"
        );
        assert!(
            !edge.components.contains_key(LIGHT),
            "an edge lights nothing"
        );
        let at = edge.transform_3d.expect("placed").position;
        assert!(at[2] < lamp_at[2], "the edge is drawn behind the lamp");
        let off = (at[0] - lamp_at[0]).hypot(at[1] - lamp_at[1]);
        let width = 4.0 * editing.canvas.per_pixel(SIZE[1]);
        assert!(off > 0.5 * width && off < 1.01 * width, "{off} vs {width}");
    }
    assert_eq!(
        opened.world.entities().count(),
        count,
        "the edges are in the canvas's copy, never in the scene"
    );
}

#[test]
fn the_handles_turn_and_scale_the_selection_in_one_undo_step_each() {
    let mut opened = opened();
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    let lamp = entity(&opened, "cover-17");
    look_at(&mut editing, &opened, lamp);
    editing.selected = Some(lamp);
    let before = transform(&opened, lamp);
    let [left, top, right, bottom] = editing
        .selection_bounds(&opened.world, &opened.extractor, SIZE)
        .expect("bounds resolve")
        .expect("the lamp is on the canvas");
    let middle = [(left + right) * 0.5, (top + bottom) * 0.5];

    // Taken above the lamp and carried round to its right, with snapping on:
    // a quarter turn clockwise.
    editing
        .take_handle(
            &opened.world,
            &opened.extractor,
            Handle::Turn,
            [middle[0], top - 60.0],
            SIZE,
            true,
        )
        .expect("the turn handle is taken");
    for pixel in [
        [middle[0] + 50.0, top - 40.0],
        [right + 80.0, middle[1] + 3.0],
    ] {
        editing
            .drag(&mut opened.world, pixel, SIZE)
            .expect("it turns");
    }
    editing.release();
    let turned = glam::Quat::from_array(transform(&opened, lamp).rotation);
    let expected = glam::Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    assert!(turned.angle_between(expected) < 1e-3, "{turned:?}");

    // Taken at the lower right and carried twice as far from the origin:
    // twice the size, still mirrored as the scene has it.
    let origin = editing
        .selection_pixel(&opened.world, SIZE)
        .expect("the lamp is on the canvas");
    let from = [right, bottom];
    let twice = [
        origin[0] + (from[0] - origin[0]) * 2.0,
        origin[1] + (from[1] - origin[1]) * 2.0,
    ];
    editing
        .take_handle(
            &opened.world,
            &opened.extractor,
            Handle::Scale,
            from,
            SIZE,
            false,
        )
        .expect("the scale handle is taken");
    editing
        .drag(&mut opened.world, twice, SIZE)
        .expect("it scales");
    editing.release();
    let scale = transform(&opened, lamp).scale;
    assert!(near(scale[0], before.scale[0] * 2.0), "{scale:?}");
    assert!(near(scale[1], before.scale[1] * 2.0), "{scale:?}");
    assert!(near(scale[2], before.scale[2]), "{scale:?}");

    editing.undo(&mut opened.world).expect("the scale undoes");
    assert_eq!(
        transform(&opened, lamp).scale.map(f32::to_bits),
        before.scale.map(f32::to_bits)
    );
    editing.undo(&mut opened.world).expect("the turn undoes");
    assert_eq!(transform(&opened, lamp), before);
}

#[test]
fn the_inspector_reports_effective_values_meanings_and_the_sprite_region() {
    let opened = opened();
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    editing.selected = Some(entity(&opened, "cover-17"));
    let shown = editing.inspector(&opened.world, &opened.extractor, &opened.assets.sheets);
    assert_eq!(shown["id"], json!("cover-17"));
    assert_eq!(shown["tile"], json!(1.0));
    assert_eq!(shown["scale"], json!([-1.5, 1.5, 1.0]));
    let sprite = shown["components"]
        .as_array()
        .expect("components")
        .iter()
        .find(|component| component["type"] == "sindri.sprite")
        .expect("the sprite is shown");
    // The scene stores only texture and layer; the rest is the engine's default.
    assert_eq!(sprite["values"]["layer"], json!(3));
    assert_eq!(sprite["values"]["unlit"], json!(false));
    assert_eq!(sprite["meanings"]["tint"]["kind"], json!("color"));
    // decor.png is four cells across; mushrooms is the third.
    assert_eq!(shown["sprite"]["texture"], json!("textures/decor.png"));
    assert_eq!(shown["sprite"]["region"], json!("mushrooms"));
    assert_eq!(shown["sprite"]["rect"], json!([0.5, 0.0, 0.25, 1.0]));
}

#[test]
fn a_slider_drag_is_one_undo_step_and_place_and_nested_fields_are_edited() {
    let mut opened = opened();
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    let lamp = entity(&opened, "cover-17");
    editing.selected = Some(lamp);
    for radius in [3.0, 3.5, 4.0] {
        editing
            .set_field(
                &mut opened.world,
                &opened.extractor,
                LIGHT,
                "radius",
                json!(radius),
                true,
            )
            .expect("the radius applies");
    }
    editing.settle();
    assert_eq!(editing.history.undo_steps().len(), 1, "one drag, one step");
    for size in [2.0, 2.5] {
        editing
            .set_place(&mut opened.world, sindri_mobile::Place::Size(size), true)
            .expect("the size applies");
    }
    editing.settle();
    editing
        .set_place(&mut opened.world, sindri_mobile::Place::Turn(90.0), false)
        .expect("the turn applies");
    assert_eq!(editing.history.undo_steps().len(), 3);
    let placed = transform(&opened, lamp);
    assert!(
        near(placed.scale[0], -2.5) && near(placed.scale[1], 2.5),
        "{placed:?}"
    );
    let shown = editing.inspector(&opened.world, &opened.extractor, &opened.assets.sheets);
    assert!(
        (shown["turn"].as_f64().expect("a turn") - 90.0).abs() < 1e-3,
        "{}",
        shown["turn"]
    );

    let wick = entity(&opened, "wick");
    editing.selected = Some(wick);
    editing
        .set_field(
            &mut opened.world,
            &opened.extractor,
            "sindri.physics2d.collider",
            "pieces.0.shape.radius",
            json!(0.6),
            false,
        )
        .expect("a nested field applies");
    let stored = &opened.world.get(wick).expect("Wick").components["sindri.physics2d.collider"];
    assert_eq!(stored["pieces"][0]["shape"]["radius"], json!(0.6));
    assert!(
        editing
            .set_field(
                &mut opened.world,
                &opened.extractor,
                "sindri.physics2d.collider",
                "pieces.0.shape.radius",
                json!("wide"),
                false,
            )
            .is_err(),
        "the schema still refuses a nested field it cannot decode"
    );
}

#[test]
fn a_tap_on_the_ground_lets_the_selection_go() {
    let opened = opened();
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    let lamp = entity(&opened, "cover-17");
    look_at(&mut editing, &opened, lamp);
    editing.selected = Some(lamp);
    // Somewhere on the field's painted ground: where the engine's picking
    // finds a tilemap, which the canvas does not select.
    let camera = editing
        .canvas
        .camera(&opened.world, &opened.extractor, SIZE)
        .expect("the camera resolves");
    let ground = (0..20_u8)
        .flat_map(|row| {
            (0..10_u8).map(move |column| {
                [
                    54.0 + 108.0 * f32::from(column),
                    60.0 + 120.0 * f32::from(row),
                ]
            })
        })
        .find(|pixel| {
            sindri_scene::picking::pick_world(
                &opened.world,
                opened.extractor.components(),
                camera.view_projection,
                [pixel[0] / SIZE[0], pixel[1] / SIZE[1]],
            )
            .expect("the pick runs")
            .and_then(|entity| opened.world.get(entity))
            .is_some_and(|data| data.components.contains_key("sindri.tilemap"))
        })
        .expect("the field's ground is on the screen");
    editing
        .tap(&opened.world, &opened.extractor, ground, SIZE)
        .expect("the tap picks");
    assert_eq!(
        editing.selected, None,
        "the ground is not selected; the lamp is let go"
    );
}
