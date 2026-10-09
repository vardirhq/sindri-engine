//! Progression begins with a small, usable crawler rather than a finished home.
mod pilot;

use pilot::Pilot;
use sindri_scene::TilemapComponent;

#[test]
fn the_starter_is_small_and_has_four_real_cargo_slots() {
    let mut pilot = Pilot::new();
    pilot.wait(0.1);
    assert!((pilot.board("capacity") - 4.0).abs() < 0.01);
    assert!((pilot.board("hull_right") - pilot.board("hull_left") - 7.0).abs() < 0.01);
    assert!((pilot.board("hull_bottom") - pilot.board("hull_top") - 9.0).abs() < 0.01);
    let deck = pilot.run.entity("deck").unwrap();
    let map = pilot
        .run
        .session
        .components()
        .get::<TilemapComponent>(&pilot.run.world, deck)
        .unwrap()
        .unwrap();
    assert_eq!(map.tile(4, 6), Some(21), "a workbench aboard");
    assert_eq!(map.tile(6, 9), Some(8), "one engine");
    for row in [8, 10] {
        for column in [4, 6] {
            assert_eq!(map.tile(column, row), Some(0), "free cargo slot");
        }
    }
    assert_eq!(map.tile(4, 14), None, "future deck is absent");
    pilot.take_the_helm();
    pilot.use_it();
    pilot.walk_deck(&[[5.5, -7.5], [6.5, -7.5], [10.0, -7.5]]);
    assert!(!pilot.flag("aboard"), "the starter's ramp works");
}

fn tile(pilot: &Pilot, column: u32, row: u32) -> Option<u32> {
    let deck = pilot.run.entity("deck").unwrap();
    pilot
        .run
        .session
        .components()
        .get::<TilemapComponent>(&pilot.run.world, deck)
        .unwrap()
        .unwrap()
        .tile(column, row)
}

fn extension(mut pilot: Pilot) {
    use pilot::{construction, salvage};
    construction::gather(&mut pilot, "trunk");
    construction::gather(&mut pilot, "trunk");
    for _ in 0..2 {
        salvage::pick_up_at_first_wreck(&mut pilot);
        construction::home(&mut pilot);
    }
    assert!((pilot.board("wood") - 2.0).abs() < 0.01);
    assert!((pilot.board("scrap") - 2.0).abs() < 0.01);
    pilot.take_the_helm();
    pilot.set_throttle(0.3);
    pilot.use_it();
    construction::bench(&mut pilot);
    let local = pilot.on_deck();
    let deck = pilot.run.entity("deck").unwrap();
    let origin = pilot.run.local(deck);
    let before = pilot.crawler_world();
    pilot.use_it();
    pilot.wait(0.5);
    assert!((pilot.board("extensions") - 1.0).abs() < 0.01);
    assert!((pilot.board("capacity") - 8.0).abs() < 0.01);
    assert!(pilot.board("crates").abs() < 0.01);
    assert!(pilot.board("wood").abs() < 0.01 && pilot.board("scrap").abs() < 0.01);
    assert!((pilot.board("structure_mass") - 18.0).abs() < 0.01);
    assert!(pilot.board("top_speed") < 4.2 * pilot.board("going") - 0.2);
    assert!(
        pilot::distance(before, pilot.crawler_world()) > 0.1,
        "kept driving"
    );
    assert!(
        pilot::distance(local, pilot.on_deck()) < 0.01,
        "crew stayed put"
    );
    assert!(
        origin
            .iter()
            .zip(pilot.run.local(deck))
            .all(|(before, after)| (before - after).abs() < f32::EPSILON),
        "stable deck origin"
    );
    assert_eq!(tile(&pilot, 5, 11), Some(0), "old stern opens");
    assert_eq!(tile(&pilot, 5, 12), Some(0), "new floor");
    assert_eq!(tile(&pilot, 5, 13), Some(1), "new stern");
    for side in ["port", "starboard"] {
        let belt = pilot.run.entity(&format!("{side}-tread-12")).unwrap();
        assert!(pilot.run.world.is_active(belt));
    }
    pilot.use_it();
    assert!((pilot.board("extensions") - 1.0).abs() < 0.01);
    construction::close(&mut pilot);
    pilot.walk_deck(&[[5.5, -12.5]]);
    assert!(pilot.flag("aboard"));
    assert!(
        (pilot.on_deck()[1] + 12.5).abs() < 0.2,
        "walked into earned space"
    );
}

#[test]
fn gathered_cargo_builds_walkable_space_while_driving_on_keys_and_touch() {
    extension(Pilot::new());
    extension(Pilot::on_a_phone());
}

#[test]
fn missing_physical_cargo_cannot_be_partly_spent_or_replaced_by_hud_counts() {
    let mut pilot = Pilot::new();
    pilot::construction::gather(&mut pilot, "trunk");
    pilot::construction::bench(&mut pilot);
    let before = tile(&pilot, 4, 10);
    pilot.run.session.blackboard_mut().set("wood", 2.0);
    pilot.run.session.blackboard_mut().set("scrap", 2.0);
    pilot.use_it();
    assert_eq!(tile(&pilot, 4, 10), before);
    assert!((pilot.board("crates") - 1.0).abs() < 0.01);
    assert!(pilot.board("extensions").abs() < 0.01);
    assert!((pilot.board("capacity") - 4.0).abs() < 0.01);
    pilot.tap(sindri_platform::Key::Escape);
    pilot.wait(0.1);
    assert!(!pilot.flag("building"));
}

// Physical hold fixtures isolate installation and chassis-limit behavior.
fn load_hold(pilot: &mut Pilot, first: u32, second: u32, kinds: [&str; 2]) {
    // Let startup initialize cargo totals before installing the fixture.
    pilot.wait(0.1);
    let deck = pilot.run.entity("deck").unwrap();
    let mut map = pilot
        .run
        .session
        .components()
        .get::<TilemapComponent>(&pilot.run.world, deck)
        .unwrap()
        .unwrap();
    for (column, row, value) in [
        (4, 10, first),
        (6, 10, first),
        (4, 8, second),
        (6, 8, second),
    ] {
        map.tiles[row * 11 + column] = Some(value);
    }
    pilot
        .run
        .world
        .get_mut(deck)
        .unwrap()
        .components
        .get_mut("sindri.tilemap")
        .unwrap()["tiles"] = serde_json::to_value(map.tiles).unwrap();
    pilot.run.session.blackboard_mut().set("crates", 4.0);
    for kind in ["wood", "fibre", "ore", "scrap"] {
        pilot.run.session.blackboard_mut().set(kind, 0.0);
    }
    for kind in kinds {
        pilot.run.session.blackboard_mut().set(kind, 2.0);
    }
    pilot.wait(0.1);
}

fn upgrades(mut pilot: Pilot) {
    use pilot::construction;
    load_hold(&mut pilot, 16, 19, ["wood", "fibre"]);
    construction::bench(&mut pilot);
    construction::next(&mut pilot);
    pilot.use_it();
    pilot.wait(0.1);
    assert!(pilot.flag("has_bunk"));
    assert_eq!(tile(&pilot, 4, 5), Some(7));
    assert!(pilot.board("crates").abs() < 0.01);
    assert!(pilot.board("wood").abs() < 0.01 && pilot.board("fibre").abs() < 0.01);
    assert!((pilot.board("structure_mass") - 4.0).abs() < 0.01);
    load_hold(&mut pilot, 16, 19, ["wood", "fibre"]);
    pilot.use_it();
    pilot.wait(0.1);
    assert!(
        (pilot.board("crates") - 4.0).abs() < 0.01,
        "duplicate bunk costs nothing"
    );
    load_hold(&mut pilot, 18, 11, ["ore", "scrap"]);
    construction::next(&mut pilot);
    pilot.use_it();
    pilot.wait(0.1);
    assert!((pilot.board("engine_level") - 1.0).abs() < 0.01);
    assert_eq!(tile(&pilot, 6, 9), Some(22));
    assert!(pilot.board("ore").abs() < 0.01 && pilot.board("scrap").abs() < 0.01);
    assert!((pilot.board("structure_mass") - 16.0).abs() < 0.01);
    assert!(
        pilot.board("top_speed") > 4.2 * pilot.board("going"),
        "engine offsets weight"
    );
    load_hold(&mut pilot, 18, 11, ["ore", "scrap"]);
    pilot.use_it();
    pilot.wait(0.1);
    assert!(
        (pilot.board("crates") - 4.0).abs() < 0.01,
        "duplicate engine costs nothing"
    );
    construction::close(&mut pilot);
}

#[test]
fn bunk_and_engine_spend_real_cargo_once_on_keys_and_touch() {
    upgrades(Pilot::new());
    upgrades(Pilot::on_a_phone());
}

#[test]
fn chassis_limit_rejects_a_fourth_extension_without_spending_cargo() {
    let mut pilot = Pilot::new();
    pilot::construction::bench(&mut pilot);
    for _ in 0..3 {
        load_hold(&mut pilot, 16, 11, ["wood", "scrap"]);
        pilot.use_it();
        pilot.wait(0.1);
    }
    assert!((pilot.board("extensions") - 3.0).abs() < 0.01);
    assert!((pilot.board("capacity") - 16.0).abs() < 0.01);
    assert_eq!(tile(&pilot, 5, 17), Some(1));
    load_hold(&mut pilot, 16, 11, ["wood", "scrap"]);
    pilot.use_it();
    pilot.wait(0.1);
    assert!((pilot.board("extensions") - 3.0).abs() < 0.01);
    assert!((pilot.board("crates") - 4.0).abs() < 0.01);
    assert!((pilot.board("structure_mass") - 54.0).abs() < 0.01);
}
