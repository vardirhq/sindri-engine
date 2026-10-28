//! Wick walks the dim fields carrying a lantern: the title opens the world,
//! the walk is the same speed in every direction, the camera follows, a
//! lantern Wick touches is lit, and the last one opens the way.

use lamplighter::Run;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

fn hold(run: &mut Run, keys: &[Key], frames: u32) {
    for key in keys {
        run.input.apply(InputEvent::KeyPressed(*key));
    }
    step(run, frames);
    for key in keys {
        run.input.apply(InputEvent::KeyReleased(*key));
    }
    step(run, 20);
}

fn position(run: &Run, id: &str) -> [f32; 2] {
    let entity = run
        .entity(id)
        .unwrap_or_else(|| panic!("{id} is in the world"));
    let at = run
        .world
        .get(entity)
        .expect("it exists")
        .transform_3d
        .expect("it has a place")
        .position;
    [at[0], at[1]]
}

#[test]
fn wick_walks_lights_the_lanterns_and_the_camera_follows() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    let start = position(&run, "wick");

    hold(&mut run, &[Key::D], 90);
    let east = position(&run, "wick");
    assert!(
        east[0] > start[0] + 4.0,
        "Wick walks east: {start:?} to {east:?}"
    );
    assert!((east[1] - start[1]).abs() < 0.2, "and only east");

    let camera = position(&run, "camera");
    assert!(
        (camera[0] - east[0]).abs() < 2.0,
        "the camera follows Wick: {camera:?} {east:?}"
    );
}

/// Walks Wick toward a point by holding the keys that point that way.
fn walk_to(run: &mut Run, target: [f32; 2]) {
    for _ in 0..900 {
        let at = position(run, "wick");
        let (dx, dy) = (target[0] - at[0], target[1] - at[1]);
        if dx.abs() < 0.3 && dy.abs() < 0.3 {
            return;
        }
        let mut keys = Vec::new();
        if dx > 0.3 {
            keys.push(Key::D);
        } else if dx < -0.3 {
            keys.push(Key::A);
        }
        if dy > 0.3 {
            keys.push(Key::W);
        } else if dy < -0.3 {
            keys.push(Key::S);
        }
        for key in &keys {
            run.input.apply(InputEvent::KeyPressed(*key));
        }
        step(run, 1);
        for key in &keys {
            run.input.apply(InputEvent::KeyReleased(*key));
        }
    }
    panic!("Wick never reached {target:?}");
}

#[test]
fn a_run_lights_every_lantern_and_the_last_opens_the_way() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    assert!(
        (run.board("total") - 3.0).abs() < f32::EPSILON,
        "three lanterns wait"
    );
    for (target, lit) in [([8.0, 0.0], 1.0), ([17.0, -3.0], 2.0), ([22.0, 8.0], 3.0)] {
        walk_to(&mut run, target);
        step(&mut run, 2);
        assert!(
            (run.board("lit") - lit).abs() < f32::EPSILON,
            "{lit} lit at {target:?}"
        );
    }
    assert!(run.board("won") > 0.0, "the last lantern opens the way");
}

/// The clip Wick's sprite is playing.
fn clip(run: &Run) -> String {
    let sprite = run.entity("wick-sprite").expect("Wick has a sprite");
    let data = run.world.get(sprite).expect("it exists");
    data.components["sindri.animation.sprite"]["playing"]
        .as_str()
        .expect("a clip")
        .to_owned()
}

#[test]
fn wick_faces_the_way_he_walks_and_stands_still_when_he_stops() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    assert_eq!(clip(&run), "idle-south", "Wick starts facing the viewer");

    for (key, facing) in [
        (Key::D, "east"),
        (Key::W, "north"),
        (Key::A, "west"),
        (Key::S, "south"),
    ] {
        run.input.apply(InputEvent::KeyPressed(key));
        step(&mut run, 20);
        assert_eq!(clip(&run), format!("walk-{facing}"), "walking {facing}");
        run.input.apply(InputEvent::KeyReleased(key));
        step(&mut run, 20);
        assert_eq!(
            clip(&run),
            format!("idle-{facing}"),
            "standing, facing {facing}"
        );
    }
}
