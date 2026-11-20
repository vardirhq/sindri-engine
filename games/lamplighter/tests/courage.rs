//! Courage (`docs/lamplighter-plan.md`, stage 4): it ebbs in the dark and goes
//! fast with the lantern dry and the gloom near, comes back by warm light, and
//! the less of it Wick has the darker the edges of the screen and the slower
//! his walk.

use lamplighter::Run;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

fn vignette(run: &Run) -> f64 {
    let environment = run.entity("environment").expect("the environment");
    run.world.get(environment).expect("it exists").components["sindri.environment"]["post_process"]
        ["vignette"]
        .as_f64()
        .expect("a number")
}

#[test]
fn courage_fails_in_the_dark_and_the_screen_closes_in() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    run.set_board("truce", 1.0);
    step(&mut run, 10);
    assert!(
        (run.board("courage") - 1.0).abs() < 1e-3,
        "he sets out brave"
    );
    let calm = vignette(&run);
    // The lantern dry, a minute in the dark.
    run.set_board("oil", 0.0);
    run.set_board("courage", 0.4);
    step(&mut run, 60 * 5);
    let left = run.board("courage");
    assert!(left < 0.3, "it ebbs fast with the lantern out: {left}");
    assert!(run.board("fear") > 0.3, "and he is afraid");
    assert!(
        vignette(&run) > calm + 0.1,
        "the edges close in: {}",
        vignette(&run)
    );
}

#[test]
fn a_fire_brings_courage_back() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    run.set_board("truce", 1.0);
    step(&mut run, 10);
    run.set_board("courage", 0.2);
    run.set_board("wood", 4.0);
    run.set_board("stone", 2.0);
    // Chosen, then set down: two presses.
    for _ in 0..2 {
        run.input.apply(InputEvent::KeyPressed(Key::F));
        step(&mut run, 2);
        run.input.apply(InputEvent::KeyReleased(Key::F));
        step(&mut run, 2);
    }
    // It takes a moment to catch.
    step(&mut run, 60 * 6);
    let back = run.board("courage");
    assert!(back > 0.5, "by the fire it returns: {back}");
    assert!(run.board("fear") < 0.01, "and the fear with it");
}
