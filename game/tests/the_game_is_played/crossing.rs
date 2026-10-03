//! The game from end to end: a walkway laid across the river, and the walk
//! over it.

use glam::Vec3;
use sindri_core::World;
use sindri_platform::{InputEvent, InputState, Key, MouseButton};
use sindri_scene::SceneExtractor;

use crate::support::*;

/// Somewhere on the picture where a click builds into `cell`.
///
/// Looked for the way a player does it: near where the cell's floor is drawn,
/// moving the pointer until the highlighted place is the one wanted. Straight
/// at the middle is not always it, because a plank already laid on the near
/// side can cover the middle of the next cell from this camera.
fn pointing_to_build(world: &World, scene: &SceneExtractor, cell: [i32; 3]) -> [f32; 2] {
    let camera = view_projection(world, scene);
    #[allow(clippy::cast_precision_loss)]
    let floor = Vec3::new(cell[0] as f32, cell[2] as f32, cell[1] as f32);
    let centre = project(floor, camera);
    for radius in 0..12 {
        for step in 0..16_u8 {
            let angle = std::f32::consts::TAU * f32::from(step) / 16.0;
            #[allow(clippy::cast_precision_loss)]
            let at = [
                centre[0] + angle.cos() * radius as f32 * 2.0,
                centre[1] + angle.sin() * radius as f32 * 2.0,
            ];
            if let Some(aim) = aim(world, scene, [at[0] / VIEWPORT.0, at[1] / VIEWPORT.1])
                && [aim.against.x, aim.against.y, aim.against.z] == cell
            {
                return at;
            }
        }
    }
    panic!("nowhere on the picture builds into {cell:?}");
}

/// The game from end to end: lay a walkway across the river by clicking its
/// water, then play and send the wanderer over it to the beacon.
#[test]
fn a_walkway_laid_across_the_river_takes_the_wanderer_to_the_beacon() {
    let (mut world, scene, mut session) = session();
    settle(&mut world, &mut session, 2);
    let start = entity_position(&world, "Wanderer");
    let beacon = entity_position(&world, "Beacon");
    #[allow(clippy::cast_possible_truncation)]
    let (row, from, to) = (
        start[2].round() as i32,
        beacon[0].round() as i32,
        start[0].round() as i32,
    );

    // Before building, the far bank is out of reach.
    let ground_now = ground(&world, &scene);
    let river: Vec<i32> = (from..to)
        .filter(|column| ground_now.block([*column, SEA, row]) == "water")
        .collect();
    assert!(
        (4..=12).contains(&river.len()),
        "a river a stock of planks can cross: {river:?}"
    );

    // Nearest the wanderer first, so each click lands on open water rather
    // than on a plank already laid in front of it.
    for column in river.iter().rev() {
        let at = pointing_to_build(&world, &scene, [*column, row, SEA + 1]);
        click(&mut world, &mut session, at, MouseButton::Left);
        let mut idle = InputState::default();
        idle.begin_frame(std::time::Duration::from_secs_f32(STEP));
        session
            .step(&mut world, &idle, VIEWPORT, STEP)
            .expect("the release steps");
        assert_eq!(
            block_at(&world, &scene, [*column, row, SEA + 1]).as_deref(),
            Some("plank-slab"),
            "column {column} is planked"
        );
    }

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

    let camera = view_projection(&world, &scene);
    #[allow(clippy::cast_precision_loss)]
    let beacon_top = Vec3::new(beacon[0], SEA as f32 + 1.0, beacon[2]);
    tap_touch(
        &mut world,
        &scene,
        &mut session,
        project(beacon_top, camera),
    );
    settle(&mut world, &mut session, 60 * 12);
    let arrived = entity_position(&world, "Wanderer");
    assert!(
        planar_distance(arrived, beacon) < 1.5,
        "the wanderer walked the planks to the beacon: at {arrived:?}, beacon at {beacon:?}"
    );
}
