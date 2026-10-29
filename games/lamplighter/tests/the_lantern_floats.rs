//! Wick's lantern is not in his sprite: it floats about him, goes round him as he turns, and
//! is drawn in front of him on the near half of its circle and behind him on the far half.

use lamplighter::Run;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

/// Where the lantern is, from Wick's feet, and its depth beside his.
fn lantern(run: &Run) -> [f32; 3] {
    let entity = run.entity("wick-lantern").expect("the lantern exists");
    let at = run
        .world
        .get(entity)
        .expect("it exists")
        .transform_3d
        .expect("a place")
        .position;
    [at[0], at[1], at[2]]
}

/// Turns Wick to face a way and lets the lantern settle.
fn face(run: &mut Run, key: Key) {
    run.input.apply(InputEvent::KeyPressed(key));
    step(run, 2);
    run.input.apply(InputEvent::KeyReleased(key));
    step(run, 90);
}

#[test]
fn the_lantern_circles_wick_as_he_turns() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    // The gloom hold still, so nothing throws Wick off the way he faces.
    run.set_board("truce", 1.0);
    let mut seen = Vec::new();
    for key in [Key::D, Key::W, Key::A, Key::S] {
        face(&mut run, key);
        seen.push(lantern(&run));
    }
    for pair in seen.windows(2) {
        let moved = (pair[0][0] - pair[1][0]).hypot(pair[0][1] - pair[1][1]);
        assert!(moved > 0.3, "it swings round to a new place: {pair:?}");
    }
    for at in &seen {
        let out = at[0].hypot((at[1] - 0.95) / 0.38 * 0.85);
        assert!((out - 0.85).abs() < 0.3, "it stays on its circle: {at:?}");
    }
}

#[test]
fn the_lantern_is_in_front_of_wick_when_near_and_behind_when_far() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    // The gloom hold still, so nothing throws Wick off the way he faces.
    run.set_board("truce", 1.0);
    // Facing east it rides at his right hand, below his middle on the screen: in front.
    face(&mut run, Key::D);
    assert!(lantern(&run)[2] > 0.0, "in front: {:?}", lantern(&run));
    // Facing north it is round the back of him.
    face(&mut run, Key::W);
    let at = lantern(&run);
    assert!(at[1] > 0.95 && at[2] < 0.0, "behind: {at:?}");
}
