//! Stage 4 of `docs/drift-plan.md`: the wreckage outside the cargo hatch.
//! Salvage is taken outside and stowed aboard; debris is solid and tumbles
//! when the diver runs into it; the derelict is solid to fly into.

use drift::Run;
use sindri_platform::Key;

const EYE: f32 = 0.6;

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f32>().sqrt()
}

#[test]
fn salvage_is_taken_outside_and_stowed_aboard() {
    // The diver floats beside the canister just outside the hatch.
    let canister = Run::open().expect("DRIFT opens").position("salvage-0");
    let mut game =
        Run::open_at([canister[0] - 1.0, canister[1] - EYE, canister[2]]).expect("DRIFT opens");
    game.wait(0.2);
    game.tap(Key::G);
    assert!(game.has_said("Drift: took salvage"), "{:?}", game.said);
    game.step();
    assert!(!game.exists("salvage-0"), "the canister is in the pack now");
    assert!(
        !game.has_said("Drift: stowed salvage"),
        "not stowed outside"
    );

    // Back aboard, in the ship's air, it is stowed.
    let cabin = game.position("spawn");
    game.place("diver", cabin);
    assert!(game.until("Drift: stowed salvage", 1.0), "{:?}", game.said);
    // Out of reach of any canister, G takes nothing.
    let taken = game.said.len();
    game.tap(Key::G);
    game.step();
    assert!(
        !game.said[taken..]
            .iter()
            .any(|line| line == "Drift: took salvage")
    );
}

#[test]
fn debris_is_solid_and_tumbles_when_the_diver_runs_into_it() {
    let target = Run::open().expect("DRIFT opens").position("debris-1");
    // Two metres short of the plate, level with it, facing it down +X.
    let mut game =
        Run::open_at([target[0] - 2.0, target[1] - EYE, target[2]]).expect("DRIFT opens");
    game.wait(0.1);
    let before = game.position("debris-1");
    game.press(Key::W);
    game.wait(1.2);
    game.release(Key::W);
    game.wait(2.0);
    let after = game.position("debris-1");
    // Left alone it drifts at most 0.05 m/s; struck, it is knocked away.
    assert!(
        dist(before, after) > 0.4,
        "the plate is knocked away: {before:?} {after:?}"
    );
    let diver = game.position("diver");
    assert!(
        dist(diver, after) > 0.3,
        "the diver did not pass through it: {diver:?} {after:?}"
    );
}

#[test]
fn the_derelict_is_solid() {
    let wreck = Run::open().expect("DRIFT opens").position("wreck");
    // Its near end faces the hatch, about four metres from its middle; fly at
    // it head on from two and a half metres short of that, on its axis.
    let mut game = Run::open_at([wreck[0] - 6.5, wreck[1] - EYE, wreck[2]]).expect("DRIFT opens");
    game.wait(0.1);
    game.press(Key::W);
    game.wait(4.0);
    game.release(Key::W);
    game.wait(2.0);
    let stopped = game.position("diver");
    assert!(
        stopped[0] < wreck[0] - 2.0,
        "the hull stops the diver short of its middle: {stopped:?} wreck at {wreck:?}"
    );
}
