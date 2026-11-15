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

    // Standing up, he keeps to the ground however far down the screen he walks,
    // and carries his lantern at his hands rather than a little up the screen.
    assert!(
        north[2].abs() < 1.0e-4,
        "Wick stands on the ground: {north:?}"
    );
    let lantern = position(&run, "wick-lantern");
    assert!(
        (0.85..1.05).contains(&(lantern[2] - north[2])),
        "the lantern is at his hands: {lantern:?}"
    );
    assert!(
        (lantern[1] - north[1]).abs() < 1.0,
        "and round him, not up the screen: {lantern:?}"
    );
}

#[test]
fn a_lamp_posts_flame_is_up_its_post_not_up_the_screen() {
    let run = Run::open_scene("wilds.scene").expect("the wilds open");
    let post = position(&run, "lantern-1");
    let flame = position(&run, "lantern-1-light");
    assert!(
        (flame[0] - post[0]).abs() < 1.0e-4 && (flame[1] - post[1]).abs() < 1.0e-4,
        "the light is at the post: {post:?} {flame:?}"
    );
    let entity = run.entity("lantern-1-light").expect("its light");
    let light = &run.world.get(entity).expect("it exists").components["sindri.light2d"];
    assert!(
        light["height"].as_f64().expect("a height") > 2.0,
        "high up it"
    );
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
