use std::time::Duration;

use sindri_core::EntityId;
use sindri_physics::{Collider2d, PhysicsPose2d, PhysicsWorld2d, RigidBody2d, RigidBodyKind};

fn entity(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}

fn bullet_position(continuous: bool) -> f32 {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    world
        .insert_body(
            entity(1),
            RigidBody2d {
                linear_velocity: [240.0, 0.0],
                continuous_collision: continuous,
                ..RigidBody2d::default()
            },
            &[Collider2d::circle(0.05)],
        )
        .unwrap();
    // Fixed colliders are swept automatically by Rapier 0.36. A kinematic
    // wall distinguishes the explicit opt-in from the discrete control.
    world
        .insert_body(
            entity(2),
            RigidBody2d {
                kind: RigidBodyKind::KinematicVelocity,
                pose: PhysicsPose2d {
                    position: [1.0, 0.0],
                    rotation: 0.0,
                },
                ..RigidBody2d::default()
            },
            &[Collider2d::rectangle([0.025, 2.0])],
        )
        .unwrap();
    world.step(Duration::from_nanos(16_666_667)).unwrap();
    world.pose(entity(1)).unwrap().position[0]
}

#[test]
fn fast_dynamic_body_stops_at_a_thin_kinematic_wall() {
    assert!(bullet_position(false) > 2.0);
    assert!(bullet_position(true) < 1.0);
}

#[test]
fn old_payloads_default_to_discrete_collision() {
    let mut payload = serde_json::to_value(RigidBody2d::default()).unwrap();
    payload
        .as_object_mut()
        .unwrap()
        .remove("continuous_collision");
    let body: RigidBody2d = serde_json::from_value(payload).unwrap();
    assert!(!body.continuous_collision);
}

#[test]
fn live_toggle_keeps_velocity_and_joint_and_rejects_wrong_kinds() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    world
        .insert_body(
            entity(1),
            RigidBody2d {
                linear_velocity: [7.0, 0.0],
                ..RigidBody2d::default()
            },
            &[Collider2d::circle(0.05)],
        )
        .unwrap();
    world
        .insert_static_collider(
            entity(2),
            PhysicsPose2d::default(),
            &[Collider2d::circle(0.05)],
        )
        .unwrap();
    world.connect_distance(entity(1), entity(2), 10.0).unwrap();
    assert!(!world.continuous_collision(entity(1)).unwrap());
    world.set_continuous_collision(entity(1), true).unwrap();
    assert!(world.continuous_collision(entity(1)).unwrap());
    assert!((world.linear_velocity(entity(1)).unwrap()[0] - 7.0).abs() < f32::EPSILON);
    assert_eq!(world.joint_count(), 1);
    world.set_continuous_collision(entity(1), false).unwrap();
    assert!(!world.continuous_collision(entity(1)).unwrap());
    assert!(world.set_continuous_collision(entity(2), true).is_err());
    assert!(world.set_continuous_collision(entity(3), true).is_err());
    assert!(!world.continuous_collision(entity(2)).unwrap());
}
