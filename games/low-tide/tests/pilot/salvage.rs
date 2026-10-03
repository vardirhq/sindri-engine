//! The salvage run, played by keys or by touch: drive to the first wreck,
//! go ashore, bring a crate of scrap home, and throw it over the side again.

use super::{Pilot, distance, frames};

/// Drives to the first wreck, salvages a crate and carries it aboard,
/// checking each step on the way as the player would see it.
pub fn pick_up_at_first_wreck(pilot: &mut Pilot) {
    let wreck = pilot.run.entity("wreck-1").expect("the first wreck");
    let wreck_at = pilot.run.position(wreck);

    // Drive at the wreck, and stop short of it: a crawler takes a while to
    // lose its way.
    pilot.take_the_helm();
    for _ in 0..frames(30.0) {
        if distance(pilot.crawler_world(), wreck_at) < 14.0 {
            break;
        }
        pilot.steer_toward(wreck_at);
    }
    pilot.set_throttle(0.0);
    pilot.wait(3.0);
    assert!(
        pilot.board("speed") < 0.05,
        "stopped: {}",
        pilot.board("speed")
    );
    pilot.use_it();

    // Down the deck, out of the hatch and down the ramp.
    pilot.walk_deck(&[[5.5, -7.5], [6.5, -7.5], [10.0, -7.5]]);
    assert!(!pilot.flag("aboard"), "ashore");
    // Round the bow, not back over the ramp, which would be going aboard.
    let off_the_bow = pilot.deck_to_world([13.0, 2.5]);
    pilot.walk_ashore(off_the_bow, 0.5);
    pilot.walk_ashore(wreck_at, 1.6);
    assert!(!pilot.flag("aboard"), "at the wreck");
    pilot.use_it();
    let carried = pilot.run.entity("carried").expect("the carried crate");
    assert!(
        pilot.run.world.is_active(carried),
        "carrying a crate of scrap"
    );
}

/// Brings the carried crate aboard and observes its weight.
pub fn salvage_the_first_wreck(pilot: &mut Pilot) {
    pick_up_at_first_wreck(pilot);
    let carried = pilot.run.entity("carried").expect("the carried crate");
    let empty_speed = 4.2;

    // Back to the foot of the ramp, then up it.
    let foot = pilot.deck_to_world([10.5, -7.5]);
    pilot.walk_ashore(foot, 0.3);
    let ramp = pilot.deck_to_world([8.4, -7.5]);
    pilot.walk_ashore(ramp, 0.2);
    pilot.wait(0.2);
    assert!(pilot.flag("aboard"), "back aboard");
    assert!(
        !pilot.run.world.is_active(carried),
        "and the crate is put down"
    );
    assert!(
        (pilot.board("scrap") - 1.0).abs() < f32::EPSILON,
        "one scrap aboard"
    );
    assert!(
        (pilot.board("crates") - 1.0).abs() < f32::EPSILON,
        "stowed in the hold"
    );
    let deck = pilot.run.entity("deck").expect("the deck");
    let hold = pilot
        .run
        .components
        .get::<sindri_scene::TilemapComponent>(&pilot.run.world, deck)
        .expect("a readable deck")
        .expect("the deck is a tilemap");
    assert_eq!(
        hold.tile(4, 10),
        Some(11),
        "the first crate sits in the stern"
    );
    let laden = pilot.board("top_speed");
    let going = pilot.board("going");
    assert!(
        laden < empty_speed * going - 0.2,
        "a heavier crawler is slower: {laden}"
    );
}

/// Walks down to the hold and throws the crate over the side.
pub fn jettison_it(pilot: &mut Pilot) {
    pilot.walk_deck(&[[6.5, -7.5], [5.5, -7.5], [5.5, -9.5], [4.5, -9.5]]);
    assert!(pilot.flag("aboard"), "still aboard, in the hold");
    if pilot.touch() {
        let drop = pilot.run.entity("drop-button").expect("the Drop button");
        pilot.step();
        assert!(pilot.run.world.is_active(drop), "Drop shows by a crate");
        pilot.press("drop-button");
        assert!(
            pilot.flag("discard_armed"),
            "first press asks for confirmation"
        );
        assert!(pilot.board("crates") > 0.5);
        pilot.press("drop-button");
    } else {
        pilot.tap(sindri_platform::Key::X);
        assert!(pilot.flag("discard_armed"));
        pilot.tap(sindri_platform::Key::X);
    }
    pilot.wait(0.2);
    assert!(pilot.board("scrap").abs() < f32::EPSILON, "over the side");
    assert!(
        pilot.board("crates").abs() < f32::EPSILON,
        "and the hold is empty"
    );
}
