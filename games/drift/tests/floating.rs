//! Stage 1 of `docs/drift-plan.md`: floating. The diver pushes off the aft
//! handhold, drifts the length of the hold with nothing but momentum, and
//! catches the handhold on the forward bulkhead.

use drift::Run;
use sindri_platform::Key;

#[test]
fn a_push_off_crosses_the_hold_to_the_far_handhold() {
    let mut game = Run::open().expect("DRIFT opens");
    game.wait(0.5);
    assert!(
        game.has_said("Drift: floating in the hold"),
        "{:?}",
        game.said
    );
    assert!(game.has_said("Drift: holding on"), "{:?}", game.said);
    let start = game.position("diver");
    game.wait(0.5);
    let held = game.position("diver");
    assert!(
        (held[0] - start[0]).abs() < 0.02,
        "holding on keeps the diver still: {start:?} {held:?}"
    );

    game.tap(Key::Space);
    assert!(game.has_said("Drift: pushed off"), "{:?}", game.said);
    // Hold F the whole way across, to catch whatever comes in reach.
    game.press(Key::F);
    game.wait(1.0);
    let drifting = game.position("diver");
    assert!(
        drifting[0] > held[0] + 2.0,
        "momentum carries the diver down the hold: {held:?} {drifting:?}"
    );
    assert!(
        drifting[1].abs() < 0.05 && drifting[2].abs() < 0.05,
        "nothing pulls the diver off the line in zero gravity: {drifting:?}"
    );
    assert!(
        game.until("Drift: caught the far handhold", 6.0),
        "{:?}",
        game.said
    );
    game.release(Key::F);
    let caught = game.position("diver");
    let far = game.position("far-hold");
    assert!(
        far[0] - caught[0] < 1.5,
        "the diver ends at the forward bulkhead: {caught:?}"
    );
    // Held there: the arm takes up the drift and the diver hangs still.
    game.press(Key::F);
    game.wait(0.5);
    let settled = game.position("diver");
    game.wait(0.5);
    let still = game.position("diver");
    assert!(
        (still[0] - settled[0]).abs() < 0.01,
        "holding on stops the drift: {settled:?} {still:?}"
    );
}

#[test]
fn thrusters_burn_propellant_and_momentum_stays_after_they_stop() {
    let mut game = Run::open().expect("DRIFT opens");
    game.wait(0.2);
    game.tap(Key::Space);
    game.wait(0.2);
    // Away from the handhold, Space is a thruster; thrust up for a second.
    game.press(Key::Space);
    game.wait(1.0);
    game.release(Key::Space);
    let after = game.position("diver");
    game.wait(0.5);
    let later = game.position("diver");
    assert!(
        later[1] > after[1] + 0.5,
        "upward speed is kept with the thrusters off: {after:?} {later:?}"
    );
    // The brake takes the drift out again.
    game.press(Key::X);
    game.wait(3.0);
    game.release(Key::X);
    let braked = game.position("diver");
    game.wait(0.5);
    let resting = game.position("diver");
    let moved = (0..3)
        .map(|i| (resting[i] - braked[i]).powi(2))
        .sum::<f32>()
        .sqrt();
    assert!(moved < 0.01, "braking stops the diver: moved {moved}");
}
