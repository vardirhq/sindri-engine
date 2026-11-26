//! Getting about with no thrusters, by whatever is to hand: crawling along a
//! bare wall, coiling a push-off for a harder shove, and throwing cargo to
//! drift the other way.

use drift::Run;
use sindri_platform::Key;

#[test]
fn a_bare_wall_is_held_and_crawled_along() {
    // Beside the hatch, off its opening, facing the forward bulkhead's plate.
    let mut game = Run::open_at([11.2, 1.4, 2.0]).expect("DRIFT opens");
    game.wait(0.3);
    game.press(Key::F);
    assert!(
        game.until("Drift: holding the surface", 1.0),
        "{:?}",
        game.said
    );
    let start = game.position("diver");
    game.press(Key::A);
    game.wait(1.0);
    game.release(Key::A);
    let along = game.position("diver");
    assert!(
        (along[2] - start[2]).abs() > 0.8,
        "crawled sideways along the wall: {start:?} {along:?}"
    );
    assert!(
        (along[0] - start[0]).abs() < 0.2,
        "neither into nor off the wall: {start:?} {along:?}"
    );
}

#[test]
fn a_coiled_push_off_is_harder_than_a_tap() {
    let distance = |hold_seconds: f32| {
        let mut game = Run::open_in_hold().expect("DRIFT opens");
        game.wait(0.5);
        game.tap(Key::U);
        game.press(Key::Space);
        game.wait(hold_seconds);
        game.release(Key::Space);
        game.step();
        assert!(game.has_said("Drift: pushed off"), "{:?}", game.said);
        let from = game.position("diver");
        game.wait(0.5);
        (
            game.position("diver")[0] - from[0],
            game.has_said("Drift: pushed off hard"),
        )
    };
    let (tapped, tapped_hard) = distance(0.02);
    let (coiled, coiled_hard) = distance(1.0);
    assert!(
        !tapped_hard && coiled_hard,
        "only the long hold is a hard push"
    );
    assert!(
        coiled > tapped * 1.7,
        "a full coil pushes about twice as hard: {coiled} against {tapped}"
    );
}

#[test]
fn throwing_cargo_drifts_the_diver_the_other_way() {
    // Floating still, just behind the small crate, facing it.
    let mut game = Run::open_at([9.2, 1.0, -1.4]).expect("DRIFT opens");
    game.wait(0.3);
    game.tap(Key::F);
    assert!(
        game.has_said("Drift: took hold of cargo"),
        "{:?}",
        game.said
    );
    game.release(Key::F);
    game.wait(0.3);
    let before = game.position("diver");
    let crate_before = game.position("crate-c");
    game.tap(Key::Space);
    assert!(game.has_said("Drift: threw cargo"), "{:?}", game.said);
    game.wait(0.5);
    let after = game.position("diver");
    let crate_after = game.position("crate-c");
    assert!(
        after[0] < before[0] - 0.4,
        "the diver drifts back: {before:?} {after:?}"
    );
    assert!(
        crate_after[0] > crate_before[0] + 1.5,
        "the crate flies forward: {crate_before:?} {crate_after:?}"
    );
}
