//! A flooded home becomes the base for a playable side-view salvage trip.
mod pilot;

use pilot::{Pilot, STEP, distance};

fn high_tide(pilot: &mut Pilot) {
    pilot.wait(0.1);
    pilot.run.scripts.blackboard_mut().set("tide_time", 270.0);
    pilot.wait(0.1);
    assert!(pilot.flag("flooded"));
    pilot.walk_deck(&[[6.5, -7.5]]);
}

fn swim(pilot: &mut Pilot, destination: [f32; 2]) {
    for _ in 0..600 {
        let at = [pilot.board("dive_x"), pilot.board("dive_y")];
        let gap = [destination[0] - at[0], destination[1] - at[1]];
        if distance(at, destination) < 0.1 {
            pilot.release();
            return;
        }
        assert!(
            pilot.flag("diving"),
            "returned before reaching {destination:?}"
        );
        let length = gap[0].hypot(gap[1]);
        pilot.push([gap[0] / length, gap[1] / length]);
        pilot.step();
    }
    panic!(
        "swimming to {destination:?} got stuck at ({}, {}), air {}",
        pilot.board("dive_x"),
        pilot.board("dive_y"),
        pilot.board("dive_air")
    );
}

fn enter(pilot: &mut Pilot) {
    high_tide(pilot);
    pilot.use_it();
    pilot.wait(0.1);
    assert!(pilot.flag("diving"));
    let dive_camera = pilot.run.entity("dive-camera").unwrap();
    let map_camera = pilot.run.entity("camera").unwrap();
    assert!(pilot.run.world.is_active(dive_camera));
    assert!(!pilot.run.world.is_active(map_camera));
}

fn trip(mut pilot: Pilot) {
    enter(&mut pilot);
    let crew_before = pilot.on_deck();
    let crawler_before = pilot.crawler_world();
    let tide_before = pilot.board("tide_time");
    swim(&mut pilot, [-1.5, -4.5]);
    swim(&mut pilot, [-1.5, -7.5]);
    swim(&mut pilot, [2.5, -7.5]);
    pilot.use_it();
    pilot.wait(0.1);
    assert!(pilot.flag("dive_loot"));
    assert!(
        pilot.board("crates").abs() < 0.01,
        "loot has not reached the hold"
    );
    swim(&mut pilot, [-1.5, -7.5]);
    swim(&mut pilot, [-1.5, 0.0]);
    pilot.use_it();
    pilot.wait(0.1);
    assert!(!pilot.flag("diving"));
    assert!(pilot.flag("aboard"));
    assert!(distance(crew_before, pilot.on_deck()) < 0.01);
    assert!(distance(crawler_before, pilot.crawler_world()) < 0.01);
    assert!(
        pilot.board("tide_time") > tide_before + 5.0,
        "tide ran while diving"
    );
    assert!((pilot.board("ore") - 1.0).abs() < 0.01);
    assert!((pilot.board("scrap") - 1.0).abs() < 0.01);
    assert!((pilot.board("crates") - 2.0).abs() < 0.01);
    let map_camera = pilot.run.entity("camera").unwrap();
    assert!(pilot.run.world.is_active(map_camera));
    // A repeat visit to this wreck cannot create a second cache.
    pilot.use_it();
    pilot.wait(0.1);
    assert!(pilot.flag("diving"));
    assert!(pilot.flag("dive_empty"));
    pilot.use_it();
    pilot.wait(0.1);
    assert!((pilot.board("crates") - 2.0).abs() < 0.01);
}

#[test]
fn a_wreck_is_dived_and_real_salvage_returns_aboard_on_keys_and_touch() {
    trip(Pilot::new());
    trip(Pilot::on_a_phone());
}

#[test]
fn the_hull_blocks_swimming_and_empty_air_recalls_without_payment() {
    let mut pilot = Pilot::new();
    enter(&mut pilot);
    swim(&mut pilot, [1.5, -3.5]);
    pilot.push([0.0, -1.0]);
    for _ in 0..180 {
        pilot.step();
    }
    pilot.release();
    assert!(pilot.board("dive_y") > -3.9, "roof blocks the diver");
    swim(&mut pilot, [-1.5, -3.5]);
    swim(&mut pilot, [-1.5, -7.5]);
    swim(&mut pilot, [2.5, -7.5]);
    pilot.use_it();
    pilot.wait(0.1);
    assert!(pilot.flag("dive_loot"));
    pilot
        .run
        .scripts
        .blackboard_mut()
        .set("dive_air", f64::from(STEP));
    pilot.wait(0.1);
    assert!(!pilot.flag("diving"));
    assert!(pilot.board("crates").abs() < 0.01);
    pilot.use_it();
    pilot.wait(0.1);
    assert!(!pilot.flag("dive_empty"), "lost loot stays below");
}

#[test]
fn low_tide_refuses_diving_and_the_ebb_recalls_a_diver() {
    let mut pilot = Pilot::new();
    pilot.walk_deck(&[[6.5, -7.5]]);
    pilot.use_it();
    pilot.wait(0.1);
    assert!(!pilot.flag("diving"));
    enter(&mut pilot);
    swim(&mut pilot, [-1.5, -2.0]);
    pilot.run.scripts.blackboard_mut().set("tide_time", 405.0);
    pilot.wait(0.1);
    assert!(!pilot.flag("diving"));
    assert!(pilot.flag("aboard"));
    assert!(pilot.board("crates").abs() < 0.01);
}

#[test]
fn phone_recall_returns_to_the_deck_without_loot() {
    let mut pilot = Pilot::on_a_phone();
    enter(&mut pilot);
    swim(&mut pilot, [-1.5, -2.0]);
    pilot.press("drop-button");
    pilot.wait(0.1);
    assert!(!pilot.flag("diving"));
    assert!(pilot.flag("aboard"));
    assert!(pilot.board("crates").abs() < 0.01);
    let stage = pilot.run.entity("dive-stage").unwrap();
    assert!(!pilot.run.world.is_active(stage));
}
