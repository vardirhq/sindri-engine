use std::time::Duration;

use sindri_core::EntityId;
use sindri_physics::{
    BodyControl2d, Collider2d, PhysicsPose2d, PhysicsWorld2d, RigidBody2d, RigidBodyKind,
};

fn entity(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}

fn world(kind: RigidBodyKind, locked: bool) -> PhysicsWorld2d {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    world
        .insert_body(
            entity(1),
            RigidBody2d {
                kind,
                lock_rotation: locked,
                ..RigidBody2d::default()
            },
            &[Collider2d::rectangle([0.5; 2])],
        )
        .unwrap();
    world
}

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1.0e-4, "{actual} != {expected}");
}

#[test]
fn forces_and_torques_accumulate_for_one_step_then_expire() {
    let mut world = world(RigidBodyKind::Dynamic, false);
    let mass = world.mass(entity(1)).unwrap();
    world.apply_force(entity(1), [2.0, 0.0]).unwrap();
    world.apply_force(entity(1), [3.0, 0.0]).unwrap();
    world.apply_torque(entity(1), 1.0).unwrap();
    world.apply_torque(entity(1), 2.0).unwrap();
    assert!(world.step(Duration::ZERO).is_err());
    world.step(Duration::from_millis(100)).unwrap();
    near(world.linear_velocity(entity(1)).unwrap()[0], 0.5 / mass);
    // Unit box: inertia = mass * (width^2 + height^2) / 12.
    near(world.angular_velocity(entity(1)).unwrap(), 1.8 / mass);
    world.step(Duration::from_millis(200)).unwrap();
    near(world.linear_velocity(entity(1)).unwrap()[0], 0.5 / mass);
    near(world.angular_velocity(entity(1)).unwrap(), 1.8 / mass);
}

#[test]
fn off_centre_impulses_translate_and_turn_immediately() {
    let mut world = world(RigidBodyKind::Dynamic, false);
    let mass = world.mass(entity(1)).unwrap();
    world
        .apply_impulse_at_point(entity(1), [2.0, 0.0], [0.0, 0.5])
        .unwrap();
    near(world.linear_velocity(entity(1)).unwrap()[0], 2.0 / mass);
    near(world.angular_velocity(entity(1)).unwrap(), -6.0 / mass);
    world.apply_angular_impulse(entity(1), 1.0).unwrap();
    near(world.angular_velocity(entity(1)).unwrap(), 0.0);
    world.step(Duration::from_millis(100)).unwrap();
    near(world.linear_velocity(entity(1)).unwrap()[0], 2.0 / mass);
}

#[test]
fn rotation_lock_preserves_translation_and_blocks_turning() {
    let mut world = world(RigidBodyKind::Dynamic, true);
    world.set_angular_velocity(entity(1), 3.0).unwrap();
    world.apply_angular_impulse(entity(1), 2.0).unwrap();
    world.apply_torque(entity(1), 5.0).unwrap();
    world
        .apply_impulse_at_point(entity(1), [1.0, 0.0], [0.0, 0.5])
        .unwrap();
    world.step(Duration::from_millis(100)).unwrap();
    near(world.angular_velocity(entity(1)).unwrap(), 0.0);
    near(world.pose(entity(1)).unwrap().rotation, 0.0);
    assert!(world.linear_velocity(entity(1)).unwrap()[0] > 0.5);
}

#[test]
fn validation_rejects_wrong_kinds_and_nonfinite_requests_without_mutation() {
    for kind in [
        RigidBodyKind::Static,
        RigidBodyKind::KinematicPosition,
        RigidBodyKind::KinematicVelocity,
    ] {
        let mut world = world(kind, false);
        for control in [
            BodyControl2d::Force([1.0, 0.0]),
            BodyControl2d::Torque(1.0),
            BodyControl2d::Impulse([1.0, 0.0]),
            BodyControl2d::AngularImpulse(1.0),
            BodyControl2d::ImpulseAtPoint {
                impulse: [1.0, 0.0],
                point: [0.0, 0.5],
            },
        ] {
            assert!(world.apply_control(entity(1), control).is_err());
        }
        assert_eq!(
            world.set_angular_velocity(entity(1), 2.0).is_ok(),
            kind == RigidBodyKind::KinematicVelocity
        );
        near(world.linear_velocity(entity(1)).unwrap()[0], 0.0);
    }
    let mut world = world(RigidBodyKind::Dynamic, false);
    for control in [
        BodyControl2d::Force([f32::NAN, 0.0]),
        BodyControl2d::Torque(f32::INFINITY),
        BodyControl2d::AngularVelocity(f32::NAN),
        BodyControl2d::AngularImpulse(f32::INFINITY),
        BodyControl2d::ImpulseAtPoint {
            impulse: [1.0, 0.0],
            point: [f32::NAN, 0.0],
        },
    ] {
        assert!(world.apply_control(entity(1), control).is_err());
    }
    world.step(Duration::from_millis(100)).unwrap();
    near(world.linear_velocity(entity(1)).unwrap()[0], 0.0);
    near(world.angular_velocity(entity(1)).unwrap(), 0.0);
    assert!(world.apply_force(entity(2), [1.0, 0.0]).is_err());
}

#[test]
fn queued_controls_replay_in_order_and_are_discarded_on_removal() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    world
        .remember_control(
            entity(1),
            RigidBodyKind::Dynamic,
            BodyControl2d::Impulse([2.0, 0.0]),
        )
        .unwrap();
    world
        .remember_linear_velocity(entity(1), [4.0, 0.0])
        .unwrap();
    world
        .remember_control(
            entity(1),
            RigidBodyKind::Dynamic,
            BodyControl2d::Impulse([1.0, 0.0]),
        )
        .unwrap();
    world
        .insert_body(
            entity(1),
            RigidBody2d::default(),
            &[Collider2d::rectangle([0.5; 2])],
        )
        .unwrap();
    near(world.linear_velocity(entity(1)).unwrap()[0], 5.0);
    world
        .remember_control(
            entity(2),
            RigidBodyKind::Dynamic,
            BodyControl2d::Force([9.0, 0.0]),
        )
        .unwrap();
    assert!(!world.remove(entity(2)));
    world
        .insert_body(
            entity(2),
            RigidBody2d {
                pose: PhysicsPose2d {
                    position: [10.0, 0.0],
                    rotation: 0.0,
                },
                ..RigidBody2d::default()
            },
            &[Collider2d::circle(0.1)],
        )
        .unwrap();
    world.step(Duration::from_millis(100)).unwrap();
    near(world.linear_velocity(entity(2)).unwrap()[0], 0.0);
    world
        .remember_control(
            entity(3),
            RigidBodyKind::Dynamic,
            BodyControl2d::Torque(1.0),
        )
        .unwrap();
    world.finish_synchronize().unwrap();
    world
        .insert_body(
            entity(3),
            RigidBody2d::default(),
            &[Collider2d::circle(0.1)],
        )
        .unwrap();
    near(world.angular_velocity(entity(3)).unwrap(), 0.0);
}
