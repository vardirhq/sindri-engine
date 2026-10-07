//! Ordered spawn controls resolve against actual collider mass at insertion.

use sindri_core::EntityId;
use sindri_physics::{
    BodyControl3d as Control, Collider3d, PhysicsWorld3d, RigidBody3d, RigidBodyKind,
};

fn id(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}
fn near(actual: [f32; 3], expected: [f32; 3]) {
    for (a, b) in actual.into_iter().zip(expected) {
        assert!((a - b).abs() < 0.003, "{a} != {b}");
    }
}

#[test]
fn replay_retains_call_order_and_uses_actual_compound_mass() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    for (entity, controls) in [
        (
            id(1),
            vec![
                Control::LinearVelocity([1.0; 3]),
                Control::Impulse([2.0; 3]),
            ],
        ),
        (
            id(2),
            vec![
                Control::Impulse([2.0; 3]),
                Control::LinearVelocity([1.0; 3]),
            ],
        ),
    ] {
        for control in controls {
            world
                .remember_control(entity, RigidBodyKind::Dynamic, control)
                .unwrap();
        }
        near(world.pending_linear_velocity(entity).unwrap(), [1.0; 3]);
        assert!(world.linear_velocity(entity).is_err());
        assert!(!world.contains(entity));
        assert!(world.pose(entity).is_err());
        world
            .insert_body(
                entity,
                RigidBody3d::default(),
                &[Collider3d::sphere(0.5), Collider3d::sphere(0.5)],
            )
            .unwrap();
    }
    let kick = 2.0 / world.mass(id(1)).unwrap();
    near(world.linear_velocity(id(1)).unwrap(), [1.0 + kick; 3]);
    near(world.linear_velocity(id(2)).unwrap(), [1.0; 3]);
    world.finish_synchronize();
    world.remove(id(1));
    world
        .insert_body(id(1), RigidBody3d::default(), &[Collider3d::sphere(0.5)])
        .unwrap();
    near(world.linear_velocity(id(1)).unwrap(), [0.0; 3]);
}

#[test]
fn invalid_requests_and_insertion_kind_fail_before_mutation_and_keep_valid_queue() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .remember_control(id(1), RigidBodyKind::Dynamic, Control::Impulse([1.0; 3]))
        .unwrap();
    for (kind, control) in [
        (RigidBodyKind::Static, Control::LinearVelocity([2.0; 3])),
        (RigidBodyKind::KinematicVelocity, Control::Impulse([2.0; 3])),
        (
            RigidBodyKind::Dynamic,
            Control::AngularVelocity([f32::NAN; 3]),
        ),
        (RigidBodyKind::Dynamic, Control::Impulse([f32::INFINITY; 3])),
    ] {
        assert!(world.remember_control(id(1), kind, control).is_err());
    }
    assert!(
        world
            .insert_body(
                id(1),
                RigidBody3d {
                    kind: RigidBodyKind::Static,
                    ..RigidBody3d::default()
                },
                &[Collider3d::sphere(0.5)]
            )
            .is_err()
    );
    assert!(world.is_empty());
    assert!(
        world
            .insert_body(id(1), RigidBody3d::default(), &[])
            .is_err()
    );
    world
        .insert_body(id(1), RigidBody3d::default(), &[Collider3d::sphere(0.5)])
        .unwrap();
    near(
        world.linear_velocity(id(1)).unwrap(),
        [1.0 / world.mass(id(1)).unwrap(); 3],
    );
    assert!(
        world
            .remember_control(
                id(1),
                RigidBodyKind::Dynamic,
                Control::LinearVelocity([99.0; 3])
            )
            .is_err()
    );
}

#[test]
fn angular_setters_locking_and_velocity_kinematic_rules_survive_replay() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    for (entity, locked) in [(id(1), false), (id(2), true)] {
        world
            .remember_control(
                entity,
                RigidBodyKind::KinematicVelocity,
                Control::AngularVelocity([1.0; 3]),
            )
            .unwrap();
        world
            .remember_control(
                entity,
                RigidBodyKind::KinematicVelocity,
                Control::AngularVelocity([2.0; 3]),
            )
            .unwrap();
        near(world.pending_angular_velocity(entity).unwrap(), [2.0; 3]);
        assert!(world.angular_velocity(entity).is_err());
        world
            .insert_body(
                entity,
                RigidBody3d {
                    kind: RigidBodyKind::KinematicVelocity,
                    lock_rotation: locked,
                    ..RigidBody3d::default()
                },
                &[Collider3d::sphere(0.5)],
            )
            .unwrap();
        near(
            world.angular_velocity(entity).unwrap(),
            if locked { [0.0; 3] } else { [2.0; 3] },
        );
    }
}

#[test]
fn removal_and_successful_synchronization_expire_unresolved_requests() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    for entity in [id(1), id(2)] {
        world
            .remember_control(
                entity,
                RigidBodyKind::Dynamic,
                Control::LinearVelocity([99.0; 3]),
            )
            .unwrap();
    }
    assert!(!world.remove(id(1)));
    assert!(world.pending_linear_velocity(id(1)).is_none());
    world.finish_synchronize();
    assert!(world.pending_linear_velocity(id(2)).is_none());
    for entity in [id(1), id(2)] {
        world
            .insert_body(entity, RigidBody3d::default(), &[Collider3d::sphere(0.5)])
            .unwrap();
        near(world.linear_velocity(entity).unwrap(), [0.0; 3]);
    }
}
