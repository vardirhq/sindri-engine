//! The lantern burns (`docs/lamplighter-plan.md`, stages 2 and 3): out in the
//! wilds its oil runs down, it gutters low and is an ember dry; resin from the
//! pack feeds it, and standing by a fire tops it up.

use lamplighter::Run;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

fn press(run: &mut Run, key: Key) {
    run.input.apply(InputEvent::KeyPressed(key));
    step(run, 2);
    run.input.apply(InputEvent::KeyReleased(key));
    step(run, 2);
}

fn beam(run: &Run) -> f64 {
    let lantern = run.entity("wick-lantern").expect("the lantern");
    run.world.get(lantern).expect("the lantern").components["sindri.light2d"]["intensity"]
        .as_f64()
        .expect("a number")
}

#[test]
fn the_lantern_runs_down_and_resin_feeds_it() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 10);
    assert!((run.board("oil") - 1.0).abs() < 0.01, "it starts full");
    let full = beam(&run);
    step(&mut run, 60 * 30);
    let left = run.board("oil");
    assert!(
        (0.8..0.9).contains(&left),
        "half a minute burns a sixth of it: {left}"
    );

    run.set_board("oil", 0.0);
    step(&mut run, 2);
    assert!(beam(&run) < 0.45 * full, "dry, it is an ember: {}", beam(&run));

    // Nothing to feed it with: R does nothing.
    press(&mut run, Key::R);
    assert!(run.board("oil") < 0.01, "no resin, no oil");
    run.set_board("resin", 1.0);
    press(&mut run, Key::R);
    assert!(
        (run.board("oil") - 0.5).abs() < 0.01,
        "a resin is half a lantern: {}",
        run.board("oil")
    );
    assert!(run.board("resin").abs() < 1e-3, "and is spent");
    assert!((run.board("fed") - 1.0).abs() < 1e-3);
    step(&mut run, 30);
    assert!(beam(&run) > 0.7 * full, "and it burns again: {}", beam(&run));
}

#[test]
fn a_fire_tops_the_lantern_up() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 10);
    run.set_board("oil", 0.3);
    run.set_board("wood", 4.0);
    run.set_board("stone", 2.0);
    press(&mut run, Key::F);
    step(&mut run, 60 * 3);
    let topped = run.board("oil");
    assert!(topped > 0.45, "by the fire it fills: {topped}");
}
