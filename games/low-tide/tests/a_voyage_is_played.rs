//! Low Tide played by a test, with the keys a person would press: the crew
//! walks a deck that drives and turns under them, goes ashore to a wreck and
//! carries its scrap home, and the voyage is won by outrunning the Tide to
//! the Rise and lost by waiting for it.

mod pilot;

use pilot::{Pilot, distance, frames, salvage};
use sindri_platform::Key;

#[test]
fn the_crew_walks_to_the_helm_and_drives_away() {
    let mut pilot = Pilot::new();
    let start = pilot.crawler_world();
    pilot.take_the_helm();
    pilot.set_throttle(1.0);
    pilot.tap(Key::E);
    assert!(!pilot.flag("at_helm"), "left the helm");

    // The lever stays where it was left, so the crawler drives on while the
    // crew walks back down the deck.
    pilot.walk_deck(&[[5.5, -3.5], [5.5, -4.5], [7.5, -4.5], [7.5, -6.0]]);
    pilot.wait(2.0);
    let moved = distance(start, pilot.crawler_world());
    assert!(moved > 8.0, "the crawler drove {moved} units");
    assert!(pilot.flag("aboard"), "and the crew is still aboard");
    let deck = pilot.on_deck();
    assert!(
        (deck[0] - 7.5).abs() < 0.2 && (deck[1] + 6.0).abs() < 0.2,
        "standing where they walked to on the deck: {deck:?}"
    );
    assert!(
        pilot.board("speed") > 3.0,
        "at speed: {}",
        pilot.board("speed")
    );
}

#[test]
fn the_deck_turns_with_the_crawler_and_the_view_turns_with_the_deck() {
    let mut pilot = Pilot::new();
    pilot.take_the_helm();
    pilot.set_throttle(0.8);
    // Hard to port for four seconds.
    pilot.push([-1.0, 0.0]);
    for _ in 0..frames(4.0) {
        pilot.step();
    }
    pilot.release();
    let heading = pilot.board("heading");
    assert!(heading > 0.6, "turned to port: {heading}");
    // At the helm the view stays north-up, the way a map does.
    assert!(
        pilot.board("view_rot").abs() < 0.05,
        "helm view is north-up"
    );

    pilot.tap(Key::E);
    pilot.wait(2.0);
    let heading = pilot.board("heading");
    let roll = pilot.board("view_rot");
    assert!(
        (roll - heading).abs() < 0.08,
        "walking the deck, the view turns with it: {roll} against {heading}"
    );

    // Down on screen is down the deck, whichever way the crawler faces.
    let from = pilot.on_deck();
    pilot.push([0.0, -1.0]);
    for _ in 0..frames(0.8) {
        pilot.step();
    }
    pilot.release();
    let to = pilot.on_deck();
    assert!(
        to[1] < from[1] - 1.5,
        "walked toward the stern: {from:?} to {to:?}"
    );
    assert!(
        (to[0] - from[0]).abs() < 0.3,
        "and straight down the deck: {to:?}"
    );

    // Everybody aboard turned with it: the crew is exactly where the floor
    // plan, turned and carried, puts them.
    let expected = pilot.deck_to_world(to);
    let actual = pilot.crew_world();
    assert!(
        distance(expected, actual) < 0.01,
        "carried by the deck: {actual:?} against {expected:?}"
    );
}

#[test]
fn a_wreck_is_salvaged_and_its_scrap_weighs_the_crawler_down() {
    let mut pilot = Pilot::new();
    salvage::salvage_the_first_wreck(&mut pilot);
    salvage::jettison_it(&mut pilot);
}

#[test]
fn a_crawler_that_waits_is_taken_by_the_tide() {
    let mut pilot = Pilot::new();
    for _ in 0..frames(40.0) {
        pilot.step();
        if pilot.flag("lost") {
            break;
        }
    }
    assert!(pilot.flag("lost"), "the Tide reached it");
    assert!(!pilot.flag("won"));
    let banner = pilot.run.entity("banner").expect("the banner");
    assert!(pilot.run.world.is_active(banner), "and says so");
}

#[test]
fn a_crawler_driven_north_outruns_the_tide_to_the_rise() {
    let mut pilot = Pilot::new();
    pilot.take_the_helm();
    let start = pilot.crawler_world();
    let mut closest = f32::MAX;
    for frame in 0..frames(150.0) {
        let at = pilot.crawler_world();
        pilot.steer_toward([start[0], at[1] + 30.0]);
        closest = closest.min(pilot.board("crawler_y") - pilot.board("tide_y"));
        if pilot.flag("won") {
            eprintln!(
                "reached the Rise after {:.1} s; the Tide came within {closest:.1} m",
                f64::from(frame) / 60.0
            );
            assert!(!pilot.flag("lost"));
            return;
        }
        assert!(!pilot.flag("lost"), "the Tide caught it at {at:?}");
    }
    panic!(
        "never reached the Rise; the crawler is at {:?}",
        pilot.crawler_world()
    );
}

#[test]
fn stepping_off_a_moving_crawler_drops_its_anchor() {
    let mut pilot = Pilot::new();
    pilot.take_the_helm();
    pilot.set_throttle(1.0);
    pilot.use_it();
    pilot.walk_deck(&[
        [5.5, -3.5],
        [5.5, -4.5],
        [7.5, -4.5],
        [7.5, -6.5],
        [12.0, -6.5],
    ]);
    assert!(!pilot.flag("aboard"), "stepped off while it was moving");
    assert!(
        pilot.board("throttle").abs() < f32::EPSILON,
        "the anchor drops"
    );
    pilot.wait(5.0);
    assert!(pilot.board("speed") < 0.01, "and it rolls to a stop");
    let gap = distance(pilot.crew_world(), pilot.crawler_world());
    assert!(gap < 12.0, "close enough to walk back to: {gap}");
}
