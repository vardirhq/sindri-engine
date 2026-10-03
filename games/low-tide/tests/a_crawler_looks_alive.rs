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

// The art generator shifts each cleat DOWN the image by its numbered phase.
// Decreasing phase therefore moves the visible upper belt toward the bow.
fn phase(pilot: &Pilot, id: &str) -> u8 {
    let entity = pilot.run.entity(id).unwrap();
    pilot
        .run
        .animations
        .sprite(entity)
        .unwrap()
        .rsplit('-')
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

fn marks_travel(pilot: &mut Pilot, id: &str, toward_bow: bool) {
    pilot.step(); // Let a direction change select its clip before sampling.
    let mut previous = phase(pilot, id);
    let mut changes = 0;
    for _ in 0..frames(0.5) {
        pilot.step();
        let next = phase(pilot, id);
        if next != previous {
            let movement = (next + 8 - previous) % 8;
            assert_eq!(
                movement,
                if toward_bow { 7 } else { 1 },
                "{id} cleats travel toward the correct end, including the loop seam"
            );
            changes += 1;
            previous = next;
        }
    }
    assert!(changes > 3, "enough cleats moved to judge their direction");
}

#[test]
fn visible_upper_belts_travel_toward_the_bow_when_driving_forward() {
    let mut pilot = Pilot::new();
    pilot.take_the_helm();
    pilot.set_throttle(1.0);
    marks_travel(&mut pilot, "port-tread", true);
    marks_travel(&mut pilot, "starboard-tread", true);
    pilot.set_throttle(0.0);
    pilot.wait(2.0);
    pilot.push([1.0, 0.0]);
    marks_travel(&mut pilot, "port-tread", true);
    marks_travel(&mut pilot, "starboard-tread", false);
    pilot.push([-1.0, 0.0]);
    marks_travel(&mut pilot, "port-tread", false);
    marks_travel(&mut pilot, "starboard-tread", true);
}
