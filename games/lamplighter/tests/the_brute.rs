//! A brute is the gloom that light does not trouble: it is bigger, it walks on through a
//! lantern's glow, and the beam that burns out an ordinary gloom leaves it whole.

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
        .expect("a place")
        .position;
    [at[0], at[1]]
}

fn alpha(run: &Run, id: &str) -> f64 {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    run.world.get(entity).expect("it exists").components["sindri.sprite"]["tint"][3]
        .as_f64()
        .expect("an alpha")
}

fn place(run: &mut Run, id: &str, at: [f32; 2]) {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    let data = run.world.get_mut(entity).expect("it exists");
    let transform = data.transform_3d.as_mut().expect("a place");
    transform.position[0] = at[0];
    transform.position[1] = at[1];
}

/// Turns Wick to face east, and puts him back at the origin.
fn face_east(run: &mut Run) {
    run.input.apply(InputEvent::KeyPressed(Key::D));
    step(run, 2);
    run.input.apply(InputEvent::KeyReleased(Key::D));
    place(run, "wick", [0.0, 0.0]);
    step(run, 20);
}

#[test]
fn the_beam_does_not_burn_a_brute_out() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    face_east(&mut run);
    place(&mut run, "brute-1", [3.2, 0.0]);
    step(&mut run, 200);
    assert!(
        run.entity("brute-1").is_some(),
        "kept in the beam, it is not burned out"
    );
    assert!(
        alpha(&run, "brute-1") > 0.9,
        "and the light does not thin it"
    );
}

#[test]
fn a_brute_walks_on_into_the_light() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    // Wick moves once, so the gloom hunt.
    run.input.apply(InputEvent::KeyPressed(Key::D));
    step(&mut run, 3);
    run.input.apply(InputEvent::KeyReleased(Key::D));
    place(&mut run, "wick", [0.0, 0.0]);
    step(&mut run, 20);
    place(&mut run, "brute-1", [6.0, 0.0]);
    step(&mut run, 2);
    let start = position(&run, "brute-1")[0];
    step(&mut run, 50);
    let later = position(&run, "brute-1")[0];
    assert!(
        start - later > 0.6,
        "it keeps coming through the glow: {start} to {later}"
    );
}

#[test]
fn a_brute_is_bigger_than_a_gloom() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    let scale = |run: &Run, id: &str| {
        let entity = run.entity(id).expect("it exists");
        run.world
            .get(entity)
            .expect("it exists")
            .transform_3d
            .expect("a place")
            .scale[0]
    };
    assert!(scale(&run, "brute-1") > scale(&run, "gloom-1") + 0.4);
}
