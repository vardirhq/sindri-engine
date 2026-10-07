//! Playing the game: pointing at the world, and changing it.
//!
//! Stated as clicks rather than as cells. A test that called `set_block` would
//! prove the volume can be written to and nothing about a person pointing at a
//! picture, so every placement here goes the whole way round -- through the
//! camera, into a ray, onto a face, back out as a cell.
//!
//! Nothing here names a coordinate the world was authored with, because the
//! world is not authored: it is generated from a seed, and a cell that is a
//! meadow under one seed is the bottom of a lake under the next. The tests ask
//! the world where the game starts and work from there.

mod crossing;
mod support;
mod tumbling;

use glam::Vec3;
use sindri_platform::{InputEvent, InputState, Key, MouseButton};

use support::*;

#[test]
fn clicking_the_top_of_the_ground_puts_a_block_on_it() {
    let (mut world, scene, mut session) = session();
    settle(&mut world, &mut session, 2);
    let (ground, at) = in_view(&world, &scene);
    let above = [ground[0], ground[1], ground[2] + 1];

    assert_eq!(block_at(&world, &scene, above), None, "the air is empty");
    click(&mut world, &mut session, at, MouseButton::Left);
    assert_eq!(
        block_at(&world, &scene, above).as_deref(),
        Some("plank-slab"),
        "a click on the ground's top lays a walkway on it"
    );
}

#[test]
fn a_click_lands_on_what_is_there_now_rather_than_on_the_world_as_generated() {
    // The difference between picking and arithmetic. The second tap names a
    // cell that did not exist when the world was built; only a ray cast
    // against the volume as it currently stands can find it.
    let (mut world, scene, mut session) = session();
    settle(&mut world, &mut session, 2);
    let (ground, at) = in_view(&world, &scene);

    // The same point on the picture, twice. The second ray meets the block the
    // first one made -- which is what this test is for, and is what a player
    // does anyway.
    //
    // Which *face* of that block it meets is not fixed, and asserting one was
    // this test's own mistake: a pixel that struck the ground's top strikes
    // whichever part of the new block now covers it, top or side, depending on
    // where in the cell it fell. What can be said is that the second tap built
    // against the first block rather than at the first block's own cell, and
    // no arithmetic on the world as generated could have found that cell.
    click(&mut world, &mut session, at, MouseButton::Left);
    let first = [ground[0], ground[1], ground[2] + 1];
    assert_eq!(
        block_at(&world, &scene, first).as_deref(),
        Some("plank-slab"),
        "the first tap builds on the ground"
    );

    click(&mut world, &mut session, at, MouseButton::Left);
    let second = [-1, 0, 1]
        .into_iter()
        .flat_map(|dx| [-1, 0, 1].map(move |dy| (dx, dy)))
        .flat_map(|(dx, dy)| [0, 1].map(move |dz| [first[0] + dx, first[1] + dy, first[2] + dz]))
        .find(|cell| {
            *cell != first && block_at(&world, &scene, *cell).as_deref() == Some("plank-slab")
        })
        .expect("the second tap built somewhere against the first block");

    // Touching it, which is what building against a face means.
    let reach =
        (second[0] - first[0]).abs() + (second[1] - first[1]).abs() + (second[2] - first[2]).abs();
    assert_eq!(
        reach, 1,
        "and built against the block the first tap made, not somewhere else: \
         {first:?} then {second:?}"
    );
}

#[test]
fn what_you_laid_comes_back_but_the_world_is_not_yours_to_carry_away() {
    let (mut world, scene, mut session) = session();
    settle(&mut world, &mut session, 2);
    let (ground, at) = in_view(&world, &scene);
    let above = [ground[0], ground[1], ground[2] + 1];

    click(&mut world, &mut session, at, MouseButton::Left);
    hold(&mut world, &mut session, at);
    assert_eq!(
        block_at(&world, &scene, above),
        None,
        "a walkway you laid comes back up"
    );

    let was = block_at(&world, &scene, ground);
    hold(&mut world, &mut session, at);
    assert_eq!(
        block_at(&world, &scene, ground),
        was,
        "the ground the world made stays where it is"
    );
}

#[test]
fn the_build_camera_looks_somewhere_else_and_the_ground_is_there() {
    let (mut world, scene, mut session) = session();
    settle(&mut world, &mut session, 2);
    let camera_before = entity_position(&world, "World Camera");

    let mut drag = InputState::default();
    drag.apply(InputEvent::TouchStarted {
        id: 11,
        x: 900.0,
        y: VIEWPORT.1 * 0.5,
    });
    session
        .step(&mut world, &drag, VIEWPORT, STEP)
        .expect("the drag starts");
    drag.begin_frame(std::time::Duration::from_secs_f32(STEP));
    drag.apply(InputEvent::TouchMoved {
        id: 11,
        x: 100.0,
        y: VIEWPORT.1 * 0.5,
    });
    session
        .step(&mut world, &drag, VIEWPORT, STEP)
        .expect("the build camera pans");

    let camera_after = entity_position(&world, "World Camera");
    assert!(
        planar_distance(camera_before, camera_after) > 0.0,
        "the regression must pan the camera"
    );
    // The world is generated wherever it is asked about, so whatever the
    // camera now frames is ground to point at rather than empty space.
    for point in [[0.1, 0.1], [0.5, 0.5], [0.1, 0.9], [0.9, 0.9]] {
        assert!(
            aim(&world, &scene, point).is_some(),
            "the newly framed world must cover visible point {point:?}, not empty space"
        );
    }
}

#[test]
fn the_river_is_something_to_build_across_rather_than_to_walk_on() {
    // The whole game in one assertion: water is not a floor, and a block laid
    // on it is. The game starts on a bank with the river to the west, so the
    // test walks out that way from the start until it finds the water.
    let (mut world, scene, mut session) = session();
    settle(&mut world, &mut session, 2);
    let camera = view_projection(&world, &scene);
    let position = entity_position(&world, "Wanderer");
    let start = sindri_scene::nearest_cell(f64::from(position[0]), f64::from(position[2]));

    let ground_now = ground(&world, &scene);
    let water = (0..20)
        .map(|step| [start.x - step, start.y])
        .find(|at| ground_now.block([at[0], SEA, at[1]]) == "water")
        .expect("the river runs west of the start");
    assert!(
        ground_now
            .surface(water[0], water[1])
            .is_some_and(|top| !top.2),
        "the river is not a floor"
    );

    let onto = [water[0], water[1], SEA + 1];
    assert_eq!(block_at(&world, &scene, onto), None, "the water is open");
    #[allow(clippy::cast_precision_loss)]
    let top = Vec3::new(water[0] as f32, SEA as f32 + 1.0, water[1] as f32);
    click(
        &mut world,
        &mut session,
        project(top, camera),
        MouseButton::Left,
    );
    assert_eq!(
        block_at(&world, &scene, onto).as_deref(),
        Some("plank-slab"),
        "clicking the water's surface lays a walkway on it"
    );
    assert!(
        ground(&world, &scene)
            .surface(water[0], water[1])
            .is_some_and(|top| top.2),
        "and the walkway is a floor"
    );
}

#[test]
#[allow(clippy::cast_precision_loss, clippy::float_cmp, clippy::too_many_lines)]
fn a_phone_tap_in_play_moves_the_target_and_the_wanderer() {
    let (mut world, scene, mut session) = session();
    settle(&mut world, &mut session, 2);

    let mut toggle = InputState::default();
    toggle.apply(InputEvent::KeyPressed(Key::Tab));
    session
        .step(&mut world, &toggle, VIEWPORT, STEP)
        .expect("play mode toggles");
    toggle.begin_frame(std::time::Duration::from_secs_f32(STEP));
    toggle.apply(InputEvent::KeyReleased(Key::Tab));
    session
        .step(&mut world, &toggle, VIEWPORT, STEP)
        .expect("tab releases");
    assert_eq!(
        ui_text(&world, &scene, "ModeLabel").text,
        "PLAY",
        "Game.playing must be true before the touch"
    );

    let target_before = entity_position(&world, "Target");
    let wanderer_before = entity_position(&world, "Wanderer");
    let camera_before = entity_position(&world, "World Camera");
    let floor = world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some("Floor"))
        .map(|(_, data)| &data.components["sindri.tile_grid"])
        .expect("the world has a floor");
    let columns = floor["columns"].as_f64().expect("columns are numeric");
    let rows = floor["rows"].as_f64().expect("rows are numeric");
    assert!(
        f64::from(wanderer_before[0]) >= 16.0
            && f64::from(wanderer_before[0]) < columns - 16.0
            && f64::from(wanderer_before[2]) >= 16.0
            && f64::from(wanderer_before[2]) < rows - 16.0,
        "the game must start at least one render chunk inside every world edge"
    );
    let initial_cell =
        sindri_scene::nearest_cell(f64::from(target_before[0]), f64::from(target_before[2]));
    let (_, at) = in_view_away_from(&world, &scene, [initial_cell.x, initial_cell.y]);

    let touched = tap_touch(&mut world, &scene, &mut session, at);
    let debug = ui_text(&world, &scene, "PlayDebug");
    assert_eq!(
        debug.values.first().copied(),
        Some(1.0),
        "Wanderer must observe Gesture.tapped (2 means Pointer.over_ui rejected it)"
    );
    assert_eq!(
        debug.values.get(1).copied(),
        Some(1.0),
        "Aim.hit must be true during the Play tap"
    );

    let target_after = entity_position(&world, "Target");
    assert_ne!(
        target_after, target_before,
        "a touch tap in Play must move the target"
    );
    assert_eq!(
        [target_after[0], target_after[2]],
        [touched[0] as f32, touched[1] as f32],
        "the target must use the release-frame voxel's logical column and row"
    );

    settle(&mut world, &mut session, 4);
    let wanderer_mid_step = entity_position(&world, "Wanderer");
    let travelled = planar_distance(wanderer_before, wanderer_mid_step);
    assert!(
        travelled > 0.0 && travelled < 0.5,
        "a rendered step must cross part of a cell instead of teleporting: {travelled}"
    );
    assert_eq!(
        entity_position(&world, "World Camera"),
        camera_before,
        "the camera must remain still while the Wanderer is inside its dead zone"
    );

    settle(&mut world, &mut session, 120);
    let wanderer_after = entity_position(&world, "Wanderer");
    assert_ne!(
        wanderer_after, wanderer_before,
        "the wanderer must continue moving toward the touch target"
    );
    assert_ne!(
        entity_position(&world, "World Camera"),
        camera_before,
        "the camera must ease after the Wanderer crosses its dead zone"
    );

    // Build edits the world; it must not quietly give the Wanderer its old
    // Beacon destination again. Switching modes may finish a grid step that
    // pathfinding already committed, but after that the Wanderer stays put.
    let mut build = InputState::default();
    build.apply(InputEvent::KeyPressed(Key::Tab));
    session
        .step(&mut world, &build, VIEWPORT, STEP)
        .expect("build mode toggles");
    build.begin_frame(std::time::Duration::from_secs_f32(STEP));
    build.apply(InputEvent::KeyReleased(Key::Tab));
    session
        .step(&mut world, &build, VIEWPORT, STEP)
        .expect("tab releases into build");
    assert_eq!(
        ui_text(&world, &scene, "ModeLabel").text,
        "BUILD",
        "the regression must actually enter Build mode"
    );

    let build_position = entity_position(&world, "Wanderer");
    settle(&mut world, &mut session, 120);
    assert_eq!(
        entity_position(&world, "Wanderer"),
        build_position,
        "the Wanderer must stay where Play left it while building"
    );
}
