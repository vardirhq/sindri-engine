//! Cosmetic belts follow driving, pivoting and flood recovery.
mod pilot;

use pilot::{Pilot, frames};
use sindri_scene::SpriteAnimationComponent;

fn belt(pilot: &Pilot, id: &str) -> SpriteAnimationComponent {
    let entity = pilot.run.entity(id).expect("a tread belt");
    pilot
        .run
        .components
        .get::<SpriteAnimationComponent>(&pilot.run.world, entity)
        .unwrap()
        .unwrap()
}

#[test]
fn the_belts_hold_at_rest_pivot_oppositely_and_resume_after_the_ebb() {
    let mut pilot = Pilot::new();
    pilot.wait(0.2);
    let port = pilot.run.entity("port-tread").unwrap();
    let still = pilot.run.animations.frame(port);
    pilot.wait(0.25);
    assert_eq!(still, pilot.run.animations.frame(port), "parked belt holds");

    pilot.take_the_helm();
    pilot.push([1.0, 0.0]);
    for _ in 0..frames(0.3) {
        pilot.step();
    }
    assert_eq!(
        belt(&pilot, "port-tread").playing.as_deref(),
        Some("forward")
    );
    assert_eq!(
        belt(&pilot, "starboard-tread").playing.as_deref(),
        Some("reverse")
    );
    assert!(belt(&pilot, "port-tread").speed > 0.0);

    let starboard = pilot.run.entity("starboard-tread").unwrap();
    let reverse = pilot.run.animations.frame(starboard);
    pilot.release();
    pilot.wait(0.2);
    assert_eq!(
        reverse,
        pilot.run.animations.frame(starboard),
        "stopping a reverse pivot holds its frame"
    );
    pilot.set_throttle(1.0);
    let mut shown = std::collections::BTreeSet::new();
    for _ in 0..frames(0.5) {
        pilot.step();
        shown.insert(pilot.run.animations.frame(port).unwrap());
    }
    assert!(shown.len() > 2, "driving visibly rolls the belt: {shown:?}");

    pilot.run.scripts.blackboard_mut().set("tide_time", 280.0);
    pilot.wait(0.1);
    assert!(pilot.flag("flooded"));
    assert!(belt(&pilot, "port-tread").speed.abs() < f32::EPSILON);
    let caught = pilot.run.animations.frame(port);
    pilot.wait(0.3);
    assert_eq!(
        caught,
        pilot.run.animations.frame(port),
        "flood freezes belt"
    );

    pilot.run.scripts.blackboard_mut().set("tide_time", 405.0);
    pilot.wait(0.2);
    assert!(!pilot.flag("flooded"));
    assert!(
        belt(&pilot, "port-tread").speed > 0.0,
        "the ebb frees the crawler"
    );
}
