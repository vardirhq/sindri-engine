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
        .components
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
