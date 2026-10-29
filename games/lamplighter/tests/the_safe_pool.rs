//! A lit lamp post is a safe pool: the gloom do not hunt Wick while he stands in one, and a
//! gloom in one, brute or not, is driven out of it and cannot wind up there.

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

fn place(run: &mut Run, id: &str, at: [f32; 2]) {
    let entity = run.entity(id).unwrap_or_else(|| panic!("{id} exists"));
    let data = run.world.get_mut(entity).expect("it exists");
    let transform = data.transform_3d.as_mut().expect("a place");
    transform.position[0] = at[0];
    transform.position[1] = at[1];
}

/// Wick moves once, so the gloom hunt, then stands at the first lamp and lights it.
fn light_the_first_lamp(run: &mut Run) {
    run.input.apply(InputEvent::KeyPressed(Key::D));
    step(run, 3);
    run.input.apply(InputEvent::KeyReleased(Key::D));
    place(run, "wick", [8.0, 0.0]);
    step(run, 30);
    assert!(run.board("lit") >= 1.0, "the lamp is lit");
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

#[test]
fn a_brute_is_driven_out_of_a_lit_lamps_pool_and_never_reaches_wick() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    light_the_first_lamp(&mut run);
    place(&mut run, "brute-1", [11.0, 0.0]);
    let before = run.board("scorched");
    step(&mut run, 240);
    let lamp = [8.0, 0.0];
    assert!(
        distance(position(&run, "brute-1"), lamp) >= 4.9,
        "it is pushed out of the pool: {:?}",
        position(&run, "brute-1")
    );
    assert!(
        run.board("scorched") - before < 0.5,
        "and it never scorches Wick"
    );
}

#[test]
fn a_gloom_does_not_hunt_wick_in_the_pool() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    light_the_first_lamp(&mut run);
    // At the pool's edge, close enough to notice him, and with no light of its own to fear
    // far from the lamp, a brute holds off.
    place(&mut run, "brute-1", [13.4, 0.0]);
    step(&mut run, 120);
    let at = position(&run, "brute-1");
    assert!(
        distance(at, [8.0, 0.0]) > 4.9,
        "it stays out of the pool: {at:?}"
    );
}

#[test]
fn outside_a_pool_a_brute_still_pounces() {
    let mut run = Run::open().expect("the project opens");
    step(&mut run, 5);
    // No lamp lit: Wick moves once, and the brute, put close, comes for him.
    run.input.apply(InputEvent::KeyPressed(Key::D));
    step(&mut run, 3);
    run.input.apply(InputEvent::KeyReleased(Key::D));
    place(&mut run, "wick", [0.0, 0.0]);
    place(&mut run, "brute-1", [2.5, 0.0]);
    step(&mut run, 2);
    let before = run.board("scorched");
    // It winds up, then pounces: a landed pounce scorches the lantern.
    step(&mut run, 120);
    assert!(
        run.board("scorched") > before,
        "it pounced and scorched his lantern"
    );
}
