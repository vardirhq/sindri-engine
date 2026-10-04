//! Walking and driving frame different amounts of the same world.
mod pilot;

use pilot::{Pilot, distance};

fn size(pilot: &Pilot) -> f64 {
    pilot
        .run
        .world
        .get(pilot.run.entity("camera").unwrap())
        .unwrap()
        .components["sindri.camera"]["vertical_size"]
        .as_f64()
        .unwrap()
}

fn views(mut pilot: Pilot) {
    pilot.wait(2.0);
    let deck_size = size(&pilot);
    pilot.take_the_helm();
    pilot.wait(2.0);
    let driving_size = size(&pilot);
    assert!(
        driving_size >= deck_size,
        "driving leaves room to look ahead"
    );
    pilot.use_it();
    pilot.walk_deck(&[[6.5, -7.5], [10.0, -7.5]]);
    pilot.wait(2.0);
    let foot_size = size(&pilot);
    assert!(
        foot_size < driving_size * 0.65,
        "walking zooms in: {foot_size}/{driving_size}"
    );
    assert!((foot_size - 15.0).abs() < 0.1);
    let camera = pilot.run.position(pilot.run.entity("camera").unwrap());
    assert!(
        distance(camera, pilot.crew_world()) < 0.1,
        "on foot follows the crew"
    );
    let ramp = pilot.deck_to_world([8.4, -7.5]);
    pilot.walk_ashore(ramp, 0.2);
    pilot.wait(2.0);
    assert!(
        size(&pilot) > foot_size + 5.0,
        "boarding restores crawler framing"
    );
    let width = size(&pilot) * f64::from(pilot.run.screen[0] / pilot.run.screen[1]);
    assert!(width >= 12.0, "both treads fit on a phone");
}

#[test]
fn walking_zoom_and_boarding_follow_keys_and_phone_controls() {
    views(Pilot::new());
    views(Pilot::on_a_phone());
}
