//! Owned 3D maximum-distance joints: ropes between body anchors.

use sindri_core::EntityId;
use sindri_physics::{
    Collider3d, DistanceJoint3d, PhysicsError, PhysicsWorld3d, RigidBody3d, RigidBodyKind,
};
use std::time::Duration;

const STEP: Duration = Duration::from_millis(10);

fn id(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}

fn same(a: f32, b: f32) -> bool {
    (a - b).abs() < f32::EPSILON
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f32>().sqrt()
}

/// A fixed post at the origin and a free ball at `x`, moving at `speed` along X.
fn post_and_ball(x: f32, speed: f32) -> PhysicsWorld3d {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_body(
            id(1),
            RigidBody3d {
                kind: RigidBodyKind::Static,
                ..RigidBody3d::default()
            },
            &[Collider3d::sphere(0.1)],
        )
        .unwrap();
    world
        .insert_body(
            id(2),
            RigidBody3d {
                position: [x, 0.0, 0.0],
                linear_velocity: [speed, 0.0, 0.0],
                ..RigidBody3d::default()
            },
            &[Collider3d::sphere(0.2)],
        )
        .unwrap();
    world
}

fn ball(world: &PhysicsWorld3d) -> [f32; 3] {
    world.pose(id(2)).unwrap().position
}

#[test]
fn a_rope_stops_a_body_at_its_length_and_lets_it_come_back() {
    let mut world = post_and_ball(1.0, 3.0);
    world
        .set_distance_joint(id(10), DistanceJoint3d::new(id(1), id(2), 2.0))
        .unwrap();
    for _ in 0..200 {
        world.step(STEP).unwrap();
        assert!(
            distance(ball(&world), [0.0; 3]) < 2.05,
            "{:?}",
            ball(&world)
        );
    }
    assert!(distance(ball(&world), [0.0; 3]) > 1.9, "drawn taut");
    // Slack: moving inwards is free.
    world.set_linear_velocity(id(2), [-1.0, 0.0, 0.0]).unwrap();
    for _ in 0..50 {
        world.step(STEP).unwrap();
    }
    assert!(ball(&world)[0] < 1.6, "{:?}", ball(&world));
}

#[test]
fn shortening_a_rope_in_place_reels_the_body_in() {
    let mut world = post_and_ball(2.0, 0.0);
    world
        .set_distance_joint(id(10), DistanceJoint3d::new(id(1), id(2), 2.0))
        .unwrap();
    world.step(STEP).unwrap();
    for length in (5..20_u8).rev().map(|tenths| f32::from(tenths) / 10.0) {
        world
            .set_distance_joint(id(10), DistanceJoint3d::new(id(1), id(2), length))
            .unwrap();
        for _ in 0..10 {
            world.step(STEP).unwrap();
        }
    }
    for _ in 0..50 {
        world.step(STEP).unwrap();
    }
    assert!(
        distance(ball(&world), [0.0; 3]) < 0.56,
        "{:?}",
        ball(&world)
    );
    assert!(
        same(world.distance_joint(id(10)).unwrap().max_distance, 0.5),
        "the live rope is the last length"
    );
}

#[test]
fn anchors_measure_from_points_on_the_bodies() {
    let mut world = post_and_ball(1.0, 3.0);
    let mut rope = DistanceJoint3d::new(id(1), id(2), 1.0);
    rope.first_anchor = [0.5, 0.0, 0.0];
    rope.second_anchor = [-0.25, 0.0, 0.0];
    world.set_distance_joint(id(10), rope).unwrap();
    for _ in 0..200 {
        world.step(STEP).unwrap();
    }
    // Anchor to anchor is one metre: the ball's centre sits 0.5 + 1 + 0.25 out.
    let x = ball(&world)[0];
    assert!((x - 1.75).abs() < 0.06, "{x}");
}

#[test]
fn removing_a_rope_or_its_body_frees_the_other_and_invalid_edits_keep_it() {
    let mut world = post_and_ball(1.0, 0.0);
    world
        .set_distance_joint(id(10), DistanceJoint3d::new(id(1), id(2), 1.5))
        .unwrap();
    assert!(matches!(
        world.set_distance_joint(id(10), DistanceJoint3d::new(id(1), id(2), 0.0)),
        Err(PhysicsError::NonPositive("distance_joint_max_distance"))
    ));
    assert!(matches!(
        world.set_distance_joint(id(10), DistanceJoint3d::new(id(1), id(1), 1.0)),
        Err(PhysicsError::JointToSelf(_))
    ));
    assert!(same(
        world.distance_joint(id(10)).unwrap().max_distance,
        1.5
    ));
    assert!(world.remove_owned_joint(id(10)));
    assert!(world.distance_joint(id(10)).is_none());
    world.set_linear_velocity(id(2), [3.0, 0.0, 0.0]).unwrap();
    for _ in 0..100 {
        world.step(STEP).unwrap();
    }
    assert!(ball(&world)[0] > 3.0, "nothing holds it now");

    // A removed endpoint takes its constraint with it.
    let mut world = post_and_ball(1.0, 0.0);
    world
        .set_distance_joint(id(10), DistanceJoint3d::new(id(1), id(2), 1.5))
        .unwrap();
    world.remove(id(1));
    assert!(world.distance_joint(id(10)).is_none());
    assert!(matches!(
        world.set_distance_joint(id(10), DistanceJoint3d::new(id(1), id(2), 1.5)),
        Err(PhysicsError::MissingEntity(_))
    ));
}

#[test]
fn a_copied_world_keeps_its_ropes() {
    let mut world = post_and_ball(1.0, 3.0);
    world
        .set_distance_joint(id(10), DistanceJoint3d::new(id(1), id(2), 2.0))
        .unwrap();
    let mut copy = world.clone();
    for _ in 0..100 {
        world.step(STEP).unwrap();
        copy.step(STEP).unwrap();
    }
    // A copy steps exactly as the original: bit for bit, not nearly.
    #[allow(clippy::float_cmp)]
    let identical = ball(&world) == ball(&copy);
    assert!(identical);
    assert!(copy.distance_joint(id(10)).is_some());
}
