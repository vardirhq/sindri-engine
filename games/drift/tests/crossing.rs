//! The crossing from the refuge to the ruin, with no thruster pack: out of
//! the airlock, onto the guide cable, and hand over hand into the ruin. And
//! the alarms flash.

use drift::Run;
use sindri_platform::Key;

/// From waking in the cabin to floating in the open outer doorway.
fn at_the_open_door() -> Run {
    let mut game = Run::open().expect("DRIFT opens");
    game.wait(1.0);
    game.press(Key::W);
    game.wait(2.6);
    game.release(Key::W);
    game.wait(0.8);
    game.tap(Key::E);
    assert!(
        game.until("Drift: outer door open", 12.0),
        "{:?}",
        game.said
    );
    game
}

#[test]
fn a_start_has_no_thruster_pack() {
    let mut game = at_the_open_door();
    // Out through the door: W walks, then the diver drifts weightless.
    game.press(Key::W);
    assert!(game.until("Drift: weightless", 4.0), "{:?}", game.said);
    game.release(Key::W);
    game.wait(0.3);
    let before = game.position("diver");
    game.press(Key::W);
    game.wait(1.0);
    game.release(Key::W);
    let after = game.position("diver");
    let thrusted = (after[0] - before[0]).abs();
    // Coasting carries the walk's speed on; with no pack W adds nothing.
    assert!(thrusted < 2.5, "W fired no jets: {before:?} {after:?}");
}

#[test]
fn hand_over_hand_along_the_guide_cable_reaches_the_ruin() {
    let mut game = at_the_open_door();
    game.press(Key::W);
    assert!(game.until("Drift: weightless", 4.0), "{:?}", game.said);
    game.release(Key::W);
    // The cable runs beside the doorway at hand height.
    game.press(Key::F);
    assert!(
        game.until("Drift: holding the cable", 2.0),
        "{:?}",
        game.said
    );
    let start = game.position("diver");
    // Pull along it, the way the view faces: towards the ruin.
    game.press(Key::W);
    game.wait(3.5);
    game.release(Key::W);
    let along = game.position("diver");
    assert!(
        along[0] > start[0] + 3.0,
        "pulled along the cable towards the ruin: {start:?} {along:?}"
    );
    assert!(along[0] > -2.4, "at the ruin's doorway: {along:?}");
}

#[test]
fn venting_air_pushes_and_costs_oxygen() {
    let mut game = at_the_open_door();
    game.press(Key::W);
    assert!(game.until("Drift: weightless", 4.0), "{:?}", game.said);
    game.release(Key::W);
    game.tap(Key::Space);
    game.step();
    assert!(game.has_said("Drift: vented air"), "{:?}", game.said);
}

#[test]
fn the_alarm_beacons_flash() {
    let mut game = Run::open().expect("DRIFT opens");
    let lit = |game: &Run| {
        let beacon = game.run.entity("alarm-0").expect("a beacon");
        game.run
            .world
            .get(beacon)
            .and_then(|data| data.components.get("sindri.light"))
            .and_then(|light| light.get("enabled"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true)
    };
    let mut seen = [false, false];
    for _ in 0..60 {
        game.step();
        seen[usize::from(lit(&game))] = true;
    }
    assert!(seen[0] && seen[1], "the beacon went both on and off");
}
