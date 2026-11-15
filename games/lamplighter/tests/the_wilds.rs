//! The wilds (`docs/lamplighter-plan.md`, stage 1): Wick walks a generated land
//! as a pixel sprite standing in 3D space, the tilted camera following him, his
//! lantern lighting the way.

use lamplighter::Run;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

fn position(run: &Run, id: &str) -> [f32; 3] {
    let entity = run
        .entity(id)
        .unwrap_or_else(|| panic!("{id} is in the wilds"));
    run.world
        .world_transform(entity)
        .expect("it has a place")
        .position
}

#[test]
fn wick_walks_the_wilds_and_the_tilted_camera_follows_from_behind() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 5);
    let start = position(&run, "wick");

    run.input.apply(InputEvent::KeyPressed(Key::W));
    step(&mut run, 60);
    run.input.apply(InputEvent::KeyReleased(Key::W));
    step(&mut run, 30);
    let north = position(&run, "wick");
    assert!(
        north[1] > start[1] + 2.0,
        "Wick walks north across the land: {start:?} to {north:?}"
    );

    // The camera stands behind him and above, and follows him there.
    let camera = position(&run, "camera");
    assert!(
        (camera[0] - north[0]).abs() < 1.5,
        "level with him: {camera:?}"
    );
    assert!(
        camera[1] < north[1] - 5.0,
        "behind him, looking ahead: {camera:?}"
    );
    assert!(camera[2] > 10.0, "and above: {camera:?}");
}

#[test]
fn what_stands_in_the_wilds_faces_the_camera() {
    let run = Run::open_scene("wilds.scene").expect("the wilds open");
    for id in ["wick", "tree-1", "boulder-1", "lantern-1", "gloom-1"] {
        let entity = run.entity(id).expect("it is in the wilds");
        let sprite = &run.world.get(entity).expect("it exists").components["sindri.sprite"];
        assert_eq!(sprite["billboard"], true, "{id} stands up");
    }
}
