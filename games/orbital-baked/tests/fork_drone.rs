//! Fork Drone is a drone: a prism that orbits the ship, and only the rounds
//! fired through it split. It used to quietly add a round to every volley,
//! which is not what anybody would guess from the name.

use orbital_baked::Run;

const STEP: f32 = 1.0 / 60.0;

fn step(run: &mut Run) {
    let notes = run.step(STEP);
    assert!(notes.is_empty(), "{notes:#?}");
    run.set_board("hp", 100_000.0);
}

fn a_run(prism: bool) -> Run {
    let mut run = Run::open().expect("the project opens");
    for _ in 0..6 {
        step(&mut run);
    }
    run.click("TitleStart");
    step(&mut run);
    if prism {
        run.set_board("comp_prism", 1.0);
    }
    step(&mut run);
    run
}

#[test]
fn the_fork_drone_is_a_prism_not_an_extra_round() {
    let run = a_run(true);
    assert_eq!(run.board("shots"), 1.0, "the volley is still one round");
    assert_eq!(
        run.count("fork_prism"),
        1,
        "and the prism is circling the ship"
    );
}

/// Two runs that differ only by the prism: every round that crosses it comes
/// out as three, so the run with it has more rounds in the air.
#[test]
fn rounds_fired_through_the_prism_split() {
    let mut plain = a_run(false);
    let mut forked = a_run(true);
    let (mut plain_rounds, mut forked_rounds) = (0, 0);
    for _ in 0..(60 * 20) {
        step(&mut plain);
        step(&mut forked);
        plain_rounds += plain.count("bullet");
        forked_rounds += forked.count("bullet");
    }
    assert!(
        forked_rounds as f32 > plain_rounds as f32 * 1.1,
        "the prism split nothing: {forked_rounds} rounds against {plain_rounds}"
    );
}
