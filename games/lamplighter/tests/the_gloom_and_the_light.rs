//! The gloom keep to the dark: they creep toward Wick where little light
//! reaches them, and hold where a light does.

use lamplighter::Run;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

fn position(run: &Run, id: &str) -> [f32; 2] {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    let at = run
        .world
        .get(entity)
        .expect("it exists")
        .transform_3d
        .expect("it has a place")
        .position;
    [at[0], at[1]]
}

fn alpha(run: &Run, id: &str) -> f64 {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    run.world.get(entity).expect("it exists").components["sindri.sprite"]["tint"][3]
        .as_f64()
        .expect("an alpha")
}

/// Holds the walk key for `frames` frames.
fn walk_east(run: &mut Run, frames: u32) {
    run.input.apply(InputEvent::KeyPressed(Key::D));
    step(run, frames);
    run.input.apply(InputEvent::KeyReleased(Key::D));
}

#[test]
fn a_gloom_creeps_through_the_dark_and_holds_at_the_edge_of_the_light() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    let start = position(&run, "gloom-1")[0];
    step(&mut run, 60);
    let early = position(&run, "gloom-1")[0];
    assert!(
        early < start - 0.3,
        "it creeps in the dark: {start} to {early}"
    );

    step(&mut run, 600);
    let settled = position(&run, "gloom-1")[0];
    let wick = position(&run, "wick")[0];
    assert!(
        settled - wick > 3.0,
        "but stops where Wick's light begins, not at him: {settled} against {wick}"
    );
    assert!(alpha(&run, "gloom-1") > 0.95, "and is solid there");
}

#[test]
fn a_gloom_far_off_does_not_notice_wick() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    let start = position(&run, "gloom-3");
    step(&mut run, 300);
    let now = position(&run, "gloom-3");
    assert!(
        (now[0] - start[0]).abs() < 0.01 && (now[1] - start[1]).abs() < 0.01,
        "it stays where it is: {start:?} to {now:?}"
    );
}

#[test]
fn a_gloom_in_a_strong_light_thins() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    // Wick walks up to it, so the lantern is on it.
    for _ in 0..40 {
        if position(&run, "gloom-1")[0] - position(&run, "wick")[0] < 2.0 {
            break;
        }
        walk_east(&mut run, 6);
    }
    step(&mut run, 40);
    assert!(
        alpha(&run, "gloom-1") < 0.7,
        "lit by the lantern it is thin: {}",
        alpha(&run, "gloom-1")
    );
}
