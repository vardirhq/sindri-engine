//! Hunger (`docs/lamplighter-plan.md`, stage 4): out in the wilds Wick
//! hungers, picks berries by walking over them and eats one from his pack once
//! he is below half fed, a cooked one by a fire going further; starving, a
//! scorch on his lantern does not mend.

use lamplighter::Run;
use sindri_platform::{InputEvent, Key};

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run, frames: u32) {
    for _ in 0..frames {
        let notes = run.step(STEP);
        assert!(notes.is_empty(), "the game reported: {notes:?}");
    }
}

#[test]
fn wick_hungers_and_eats_a_berry_when_half_fed() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    run.set_board("truce", 1.0);
    step(&mut run, 10);
    assert!((run.board("food") - 1.0).abs() < 0.01, "he sets out fed");
    step(&mut run, 60 * 30);
    let left = run.board("food");
    assert!(
        (0.85..0.95).contains(&left),
        "half a minute takes a tenth: {left}"
    );

    run.set_board("berries", 2.0);
    run.set_board("food", 0.45);
    step(&mut run, 2);
    assert!((run.board("berries") - 1.0).abs() < 1e-3, "one is eaten");
    let fed = run.board("food");
    assert!((0.65..0.75).contains(&fed), "a berry is a quarter: {fed}");
    step(&mut run, 2);
    assert!(
        (run.board("berries") - 1.0).abs() < 1e-3,
        "and no more above half"
    );
}

#[test]
fn a_berry_by_a_fire_is_cooked_and_goes_further() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    run.set_board("truce", 1.0);
    step(&mut run, 10);
    run.set_board("wood", 4.0);
    run.set_board("stone", 2.0);
    run.input.apply(InputEvent::KeyPressed(Key::F));
    step(&mut run, 2);
    run.input.apply(InputEvent::KeyReleased(Key::F));
    step(&mut run, 10);
    run.set_board("berries", 1.0);
    run.set_board("food", 0.2);
    step(&mut run, 2);
    let fed = run.board("food");
    assert!(
        (0.65..0.75).contains(&fed),
        "cooked, a berry is half: {fed}"
    );
}

#[test]
fn starving_a_scorch_does_not_mend() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    run.set_board("truce", 1.0);
    step(&mut run, 10);
    run.set_board("food", 0.0);
    run.set_board("scorch", 0.5);
    step(&mut run, 60 * 3);
    assert!(
        (run.board("scorch") - 0.5).abs() < 1e-3,
        "it stays scorched"
    );
    run.set_board("food", 1.0);
    step(&mut run, 60 * 3);
    assert!(run.board("scorch") < 0.4, "fed, it mends");
}

/// Out in the wilds a lantern put out starts the wilds over, not the field the
/// game used to begin in, and what Wick carried is gone.
#[test]
fn a_lantern_put_out_in_the_wilds_starts_the_wilds_over() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    step(&mut run, 10);
    run.set_board("berries", 3.0);
    run.set_board("lost", 1.0);
    let mut again = false;
    for _ in 0..300 {
        step(&mut run, 1);
        if run.board("lost") < 0.5 {
            again = true;
            break;
        }
    }
    assert!(again, "it starts again");
    assert_eq!(run.session.scene(), Some("wilds.scene"), "in the wilds");
    step(&mut run, 2);
    assert!(run.board("berries").abs() < 1e-3, "with an empty pack");
}
