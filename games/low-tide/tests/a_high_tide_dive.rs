//! Swim to permanent water, dive a wreck and haul floated salvage home.
mod pilot;

use pilot::{Pilot, STEP, distance};

fn walk_to(pilot: &mut Pilot, target: [f32; 2], close: f32) {
    while distance(pilot.crew_world(), target) > 10.0 && !pilot.flag("aboard") {
        let at = pilot.crew_world();
        let length = distance(at, target);
        let next = [
            at[0] + (target[0] - at[0]) * 10.0 / length,
            at[1] + (target[1] - at[1]) * 10.0 / length,
        ];
        pilot.walk_ashore(next, 0.5);
    }
    pilot.walk_ashore(target, close);
}

fn swim_to_site(pilot: &mut Pilot) -> [f32; 2] {
    pilot.wait(0.1);
    let centre = pilot.crawler_world();
    let site = pilot
        .run
        .world
        .entities()
        .filter(|(_, data)| {
            data.components.get("sindri.tags").is_some_and(|tags| {
                tags["tags"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|tag| tag.as_str() == Some("sunken-wreck"))
            })
        })
        .map(|(entity, _)| pilot.run.position(entity))
        .min_by(|a, b| distance(*a, centre).total_cmp(&distance(*b, centre)))
        .expect("permanent water has a marked wreck near home");
    pilot.walk_deck(&[[6.5, -7.5], [10.0, -7.5]]);
    let off_the_bow = pilot.deck_to_world([13.0, 2.5]);
    pilot.walk_ashore(off_the_bow, 0.5);
    walk_to(pilot, site, 0.3);
    pilot.wait(0.1);
    assert!(pilot.flag("swimming"));
    assert!(!pilot.flag("aboard"));
    assert_eq!(pilot.surface_under(site), "brine");
    site
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

fn enter(pilot: &mut Pilot) -> [f32; 2] {
    let site = swim_to_site(pilot);
    pilot.use_it();
    pilot.wait(0.1);
    assert!(pilot.flag("diving"));
    let dive_camera = pilot.run.entity("dive-camera").unwrap();
    let map_camera = pilot.run.entity("camera").unwrap();
    assert!(pilot.run.world.is_active(dive_camera));
    assert!(!pilot.run.world.is_active(map_camera));
    site
}

fn trip(mut pilot: Pilot) {
    let site = enter(&mut pilot);
    let crew_before = pilot.crew_world();
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
    assert!(!pilot.flag("aboard"));
    assert!(pilot.flag("swimming"));
    assert!(distance(crew_before, pilot.crew_world()) < 0.01);
    assert!(distance(crawler_before, pilot.crawler_world()) < 0.01);
    assert!(
        pilot.board("tide_time") > tide_before + 5.0,
        "tide ran while diving"
    );
    assert!(
        pilot.board("crates").abs() < 0.01,
        "salvage is still on the float"
    );
    assert!(pilot.board("carried_kind") > 0.5);
    let map_camera = pilot.run.entity("camera").unwrap();
    assert!(pilot.run.world.is_active(map_camera));
    // Swim back, then haul the ore up the same ramp used for shore cargo.
    let foot = pilot.deck_to_world([10.5, -7.5]);
    walk_to(&mut pilot, foot, 0.3);
    let ramp = pilot.deck_to_world([8.4, -7.5]);
    pilot.walk_ashore(ramp, 0.2);
    assert!(pilot.flag("aboard"));
    assert!((pilot.board("ore") - 1.0).abs() < 0.01);
    assert!((pilot.board("crates") - 1.0).abs() < 0.01);
    assert!(!pilot.flag("swimming"));
    // Moving the host's observer unloads the marker; it must retain its cache
    // when the same site streams back. The actual trip is played above.
    let marker = pilot
        .run
        .world
        .entities()
        .find(|(entity, data)| {
            data.name.as_deref() == Some("Submerged wreck")
                && distance(pilot.run.position(*entity), site) < 0.1
        })
        .map(|(entity, _)| entity)
        .unwrap();
    let original = pilot
        .run
        .world
        .get(pilot.crawler)
        .unwrap()
        .transform_3d
        .unwrap();
    pilot
        .run
        .world
        .get_mut(pilot.crawler)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position[0] += 300.0;
    pilot.wait(1.2);
    assert!(pilot.run.world.get(marker).is_none(), "marker streamed out");
    pilot.run.world.get_mut(pilot.crawler).unwrap().transform_3d = Some(original);
    pilot.wait(1.2);
    swim_to_site(&mut pilot);
    pilot.use_it();
    pilot.wait(0.1);
    assert!(pilot.flag("diving"));
    assert!(pilot.flag("dive_empty"));
    pilot.use_it();
    pilot.wait(0.1);
    assert!((pilot.board("crates") - 1.0).abs() < 0.01);
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
fn the_ramp_refuses_diving_and_permanent_water_stays_divable_through_the_ebb() {
    let mut pilot = Pilot::new();
    pilot.walk_deck(&[[6.5, -7.5]]);
    pilot.use_it();
    pilot.wait(0.1);
    assert!(!pilot.flag("diving"));
    enter(&mut pilot);
    swim(&mut pilot, [-1.5, -2.0]);
    pilot.run.scripts.blackboard_mut().set("tide_time", 280.0);
    pilot.wait(0.1);
    assert!(pilot.flag("diving"));
    pilot.run.scripts.blackboard_mut().set("tide_time", 405.0);
    pilot.wait(0.1);
    assert!(
        pilot.flag("diving"),
        "permanent water does not drain with the flats"
    );
    pilot.tap(sindri_platform::Key::Escape);
    pilot.wait(0.1);
    assert!(!pilot.flag("diving"));
    assert!(!pilot.flag("aboard"));
    assert!(pilot.board("crates").abs() < 0.01);
}

#[test]
fn phone_surface_returns_to_swimming_without_loot() {
    let mut pilot = Pilot::on_a_phone();
    enter(&mut pilot);
    swim(&mut pilot, [-1.5, -2.0]);
    pilot.press("drop-button");
    pilot.wait(0.1);
    assert!(!pilot.flag("diving"));
    assert!(!pilot.flag("aboard"));
    assert!(pilot.flag("swimming"));
    assert!(pilot.board("crates").abs() < 0.01);
    let stage = pilot.run.entity("dive-stage").unwrap();
    assert!(!pilot.run.world.is_active(stage));
}

#[test]
fn temporarily_flooded_flats_allow_swimming_but_not_wreck_diving() {
    let mut pilot = Pilot::new();
    pilot.walk_deck(&[[6.5, -7.5], [10.0, -7.5]]);
    let ground = pilot.surface_under(pilot.crew_world());
    assert_ne!(ground, "brine");
    pilot.run.scripts.blackboard_mut().set("tide_time", 280.0);
    pilot.wait(0.1);
    assert!(pilot.flag("swimming"));
    pilot.use_it();
    pilot.wait(0.1);
    assert!(!pilot.flag("diving"));
    pilot.run.scripts.blackboard_mut().set("tide_time", 405.0);
    pilot.wait(0.1);
    assert!(!pilot.flag("swimming"));
    assert_eq!(pilot.surface_under(pilot.crew_world()), ground);
}
