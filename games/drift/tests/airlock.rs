//! Stage 3 of `docs/drift-plan.md`: the airlock. The diver wakes in the
//! cabin under the ship's gravity, walks into the airlock, cycles it, steps
//! out weightless into the hold, comes back in and cycles it again; the suit
//! breathes in vacuum and refills aboard, and running out of air ends aboard.

use drift::Run;
use sindri_platform::Key;

fn said_after(game: &Run, from: usize, line: &str) -> bool {
    game.said[from..].iter().any(|said| said == line)
}

#[test]
fn the_diver_walks_out_through_the_airlock_and_back_in() {
    let mut game = Run::open().expect("DRIFT opens");
    game.wait(1.0);
    assert!(
        game.has_said("Drift: awake in the cabin"),
        "{:?}",
        game.said
    );
    let standing = game.position("diver");
    game.wait(0.5);
    let still = game.position("diver");
    assert!(
        (still[1] - standing[1]).abs() < 0.01 && still[1] < -1.6,
        "standing on the cabin deck under gravity: {standing:?} {still:?}"
    );

    // Walk forward through the open inner door into the chamber.
    game.press(Key::W);
    game.wait(2.6);
    game.release(Key::W);
    game.wait(0.8);
    let chamber = game.position("diver");
    assert!(
        chamber[0] > -9.4 && chamber[0] < -7.6,
        "in the airlock chamber: {chamber:?}"
    );
    assert!(chamber[1] < -1.6, "still on the deck: {chamber:?}");

    game.tap(Key::E);
    assert!(
        game.has_said("Drift: airlock cycling out"),
        "{:?}",
        game.said
    );
    assert!(game.until("Drift: outer door open", 6.0), "{:?}", game.said);
    let inner = game.position("inner-door");
    assert!(
        (inner[2] - 1.5).abs() < 0.01,
        "the inner door is shut while the outer is open: {inner:?}"
    );

    // Step out: past the outer door there is no gravity.
    let out = game.said.len();
    game.press(Key::W);
    assert!(game.until("Drift: weightless", 3.0), "{:?}", game.said);
    game.release(Key::W);
    game.wait(1.0);
    let floating = game.position("diver");
    game.wait(1.0);
    let later = game.position("diver");
    assert!(
        later[0] > floating[0] + 1.0,
        "walking speed carries on as drift: {floating:?} {later:?}"
    );
    assert!(
        (later[1] - floating[1]).abs() < 0.05,
        "nothing pulls the diver down outside: {floating:?} {later:?}"
    );
    assert!(said_after(&game, out, "Drift: weightless"));

    // Stop, and thrust back in through the outer door.
    game.press(Key::X);
    game.wait(2.0);
    game.release(Key::X);
    game.press(Key::S);
    assert!(game.until("Drift: gravity", 6.0), "{:?}", game.said);
    game.release(Key::S);
    game.press(Key::X);
    game.wait(0.3);
    game.release(Key::X);
    game.wait(1.5);
    let back = game.position("diver");
    assert!(
        back[0] < -7.4 && back[1] < -1.6,
        "back on the chamber deck: {back:?}"
    );

    game.tap(Key::E);
    assert!(
        game.has_said("Drift: airlock cycling in"),
        "{:?}",
        game.said
    );
    assert!(game.until("Drift: inner door open", 6.0), "{:?}", game.said);
    let outer = game.position("outer-door");
    assert!(
        (outer[2] - 1.5).abs() < 0.01,
        "the outer door is shut again: {outer:?}"
    );
}

#[test]
fn a_cycle_waits_for_each_door_and_the_pressure() {
    let mut game = Run::open().expect("DRIFT opens");
    game.wait(0.5);
    game.press(Key::W);
    game.wait(2.6);
    game.release(Key::W);
    game.wait(0.8);
    game.tap(Key::E);
    // Half a second in, the inner door is closing and the outer still shut.
    game.wait(0.5);
    let inner = game.position("inner-door")[2];
    let outer = game.position("outer-door")[2];
    assert!(inner < 1.45 && inner > 0.3, "inner door part way: {inner}");
    assert!((outer - 1.5).abs() < 0.01, "outer door shut: {outer}");
    // While it pumps down both doors are shut; a second press changes nothing.
    game.wait(1.5);
    game.tap(Key::E);
    assert_eq!(
        game.said
            .iter()
            .filter(|line| line.starts_with("Drift: airlock cycling"))
            .count(),
        1,
        "{:?}",
        game.said
    );
    assert!((game.position("inner-door")[2] - 1.5).abs() < 0.01);
    assert!((game.position("outer-door")[2] - 1.5).abs() < 0.01);
    assert!(!game.has_said("Drift: outer door open"));
    assert!(game.until("Drift: outer door open", 4.0), "{:?}", game.said);
}

#[test]
fn outside_the_suit_runs_out_of_air_and_the_diver_wakes_aboard() {
    let mut game = Run::open_in_hold().expect("DRIFT opens");
    game.wait(0.5);
    assert!(
        game.has_said("Drift: floating in the hold"),
        "{:?}",
        game.said
    );
    assert!(game.until("Drift: oxygen low", 110.0), "{:?}", game.said);
    assert!(game.until("Drift: out of air", 40.0), "{:?}", game.said);
    game.wait(1.0);
    let woke = game.position("diver");
    assert!(
        woke[0] < -12.0 && woke[1] < -1.6,
        "the diver wakes standing in the cabin: {woke:?}"
    );
    // Aboard, the suit fills again and the diver does not black out twice.
    game.wait(10.0);
    let blackouts = game
        .said
        .iter()
        .filter(|line| *line == "Drift: out of air")
        .count();
    assert_eq!(blackouts, 1, "{:?}", game.said);
}
