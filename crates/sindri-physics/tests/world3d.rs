//! Standalone 3D engine semantics; scene/Decay/game integration is separate.

use sindri_core::EntityId;
use sindri_physics::{
    Collider3d, CollisionLayers, PhysicsError, PhysicsEventKind, PhysicsPose3d, PhysicsWorld3d,
    RigidBody3d, RigidBodyKind,
};
use std::time::Duration;

const STEP: Duration = Duration::from_millis(10);

fn id(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}
fn at(position: [f32; 3]) -> PhysicsPose3d {
    PhysicsPose3d {
        position,
        ..PhysicsPose3d::default()
    }
}
fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.003, "{actual} != {expected}");
}

fn near3(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        near(actual, expected);
    }
}

#[test]
fn fixed_steps_integrate_all_three_axes_and_gravity_scale() {
    let mut world = PhysicsWorld3d::new([1.0, -2.0, 3.0]).unwrap();
    world
        .insert_body(
            id(1),
            RigidBody3d {
                gravity_scale: 2.0,
                ..RigidBody3d::default()
            },
            &[Collider3d::sphere(0.3)],
        )
        .unwrap();
    for _ in 0..100 {
        world.step(STEP).unwrap();
    }
    let position = world.pose(id(1)).unwrap().position;
    assert!(position[0] > 0.9 && position[1] < -1.9 && position[2] > 2.9);
    let velocity = world.linear_velocity(id(1)).unwrap();
    for (actual, expected) in velocity.into_iter().zip([2.0, -4.0, 6.0]) {
        near(actual, expected);
    }
    world.set_gravity([0.0; 3]).unwrap();
    near3(world.gravity(), [0.0; 3]);
}

#[test]
fn all_shapes_land_on_real_solid_geometry_and_publish_collision_events() {
    for collider in [
        Collider3d::sphere(0.5),
        Collider3d::cuboid([0.3, 0.5, 0.4]),
        Collider3d::capsule(0.25, 0.25),
    ] {
        let mut world = PhysicsWorld3d::new([0.0, -9.81, 0.0]).unwrap();
        world
            .insert_static_collider(id(1), at([0.0; 3]), &[Collider3d::cuboid([4.0, 0.25, 4.0])])
            .unwrap();
        world
            .insert_body(
                id(10),
                RigidBody3d {
                    position: [0.0, 3.0, 0.0],
                    lock_rotation: true,
                    ..RigidBody3d::default()
                },
                &[collider],
            )
            .unwrap();
        let mut contact = false;
        for _ in 0..250 {
            contact |= world.step(STEP).unwrap().iter().any(|event| {
                event.first == id(1)
                    && event.second == id(10)
                    && event.kind == PhysicsEventKind::CollisionStarted
            });
        }
        assert!(contact, "solid contact for {collider:?}");
        assert!((world.pose(id(10)).unwrap().position[1] - 0.75).abs() < 0.06);
    }
}

#[test]
fn body_and_piece_quaternions_and_local_offsets_determine_collision_geometry() {
    let half = std::f32::consts::FRAC_1_SQRT_2;
    let mut world = PhysicsWorld3d::new([0.0, -9.81, 0.0]).unwrap();
    let rotation = [0.0, half, 0.0, half];
    let pole = Collider3d {
        offset: [0.0, 0.0, 2.0],
        rotation: [0.0, 0.0, half, half],
        ..Collider3d::cuboid([2.0, 0.1, 0.1])
    };
    world
        .insert_static_collider(
            id(1),
            PhysicsPose3d {
                rotation,
                ..PhysicsPose3d::default()
            },
            &[pole],
        )
        .unwrap();
    for (actual, expected) in world
        .pose(id(1))
        .unwrap()
        .rotation
        .into_iter()
        .zip(rotation)
    {
        near(actual, expected);
    }
    world
        .insert_body(
            id(2),
            RigidBody3d {
                position: [2.0, 4.0, 0.0],
                ..RigidBody3d::default()
            },
            &[Collider3d::sphere(0.3)],
        )
        .unwrap();
    for _ in 0..80 {
        world.step(STEP).unwrap();
    }
    // Local Z offset becomes world X; the local X pole is turned upright.
    assert!((world.pose(id(2)).unwrap().position[1] - 2.3).abs() < 0.06);
}

#[test]
fn sensor_enter_exit_events_use_sorted_entity_pairs_and_masks() {
    for admitted in [false, true] {
        let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
        let sensor = Collider3d {
            sensor: true,
            layers: CollisionLayers::new(2, 1),
            ..Collider3d::sphere(1.0)
        };
        let actor = Collider3d {
            layers: CollisionLayers::new(1, if admitted { 2 } else { 0 }),
            ..Collider3d::sphere(0.3)
        };
        world
            .insert_static_collider(id(8), at([0.0; 3]), &[sensor])
            .unwrap();
        world
            .insert_body(id(2), RigidBody3d::default(), &[actor])
            .unwrap();
        let events = world.step(STEP).unwrap();
        if !admitted {
            assert!(events.is_empty());
            continue;
        }
        assert!(events.iter().any(|event| event.first == id(2)
            && event.second == id(8)
            && event.kind == PhysicsEventKind::SensorEntered));
        near(world.pose(id(2)).unwrap().position[0], 0.0);
        world.move_to(id(2), at([4.0, 0.0, 0.0])).unwrap();
        assert!(
            world
                .step(STEP)
                .unwrap()
                .iter()
                .any(|event| event.kind == PhysicsEventKind::SensorExited)
        );
    }
}

#[test]
fn velocity_kinematics_and_position_targets_use_the_fixed_step() {
    let mut world = PhysicsWorld3d::new([0.0, -9.81, 0.0]).unwrap();
    for (entity, kind) in [
        (id(1), RigidBodyKind::KinematicVelocity),
        (id(2), RigidBodyKind::KinematicPosition),
    ] {
        world
            .insert_body(
                entity,
                RigidBody3d {
                    kind,
                    ..RigidBody3d::default()
                },
                &[Collider3d::sphere(0.1)],
            )
            .unwrap();
    }
    world.set_linear_velocity(id(1), [1.0, 2.0, 3.0]).unwrap();
    world
        .set_kinematic_target(id(2), at([3.0, 4.0, 5.0]))
        .unwrap();
    near3(world.pose(id(2)).unwrap().position, [0.0; 3]);
    world.step(Duration::from_millis(100)).unwrap();
    for (actual, expected) in world
        .pose(id(1))
        .unwrap()
        .position
        .into_iter()
        .zip([0.1, 0.2, 0.3])
    {
        near(actual, expected);
    }
    near3(world.pose(id(2)).unwrap().position, [3.0, 4.0, 5.0]);
    world.move_to(id(2), at([6.0, 7.0, 8.0])).unwrap();
    near3(world.pose(id(2)).unwrap().position, [3.0, 4.0, 5.0]);
    world.step(STEP).unwrap();
    near3(world.pose(id(2)).unwrap().position, [6.0, 7.0, 8.0]);
}

#[test]
fn impulses_use_compound_mass_and_teleports_preserve_velocity() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let sphere = Collider3d::sphere(0.5);
    world
        .insert_body(id(1), RigidBody3d::default(), &[sphere])
        .unwrap();
    let one_mass = world.mass(id(1)).unwrap();
    world
        .insert_body(
            id(2),
            RigidBody3d::default(),
            &[
                Collider3d {
                    offset: [-2.0, 0.0, 0.0],
                    ..sphere
                },
                Collider3d {
                    offset: [2.0, 0.0, 0.0],
                    ..sphere
                },
            ],
        )
        .unwrap();
    near(world.mass(id(2)).unwrap(), one_mass * 2.0);
    world.apply_impulse(id(2), [0.0, 0.0, 2.0]).unwrap();
    near(world.linear_velocity(id(2)).unwrap()[2], 1.0 / one_mass);
    let velocity = world.linear_velocity(id(2)).unwrap();
    world.move_to(id(2), at([10.0, 20.0, 30.0])).unwrap();
    near3(world.pose(id(2)).unwrap().position, [10.0, 20.0, 30.0]);
    near3(world.linear_velocity(id(2)).unwrap(), velocity);
}

#[test]
fn angular_velocity_turns_quaternions_and_rotation_lock_keeps_zero() {
    for locked in [false, true] {
        let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
        world
            .insert_body(
                id(1),
                RigidBody3d {
                    lock_rotation: locked,
                    angular_velocity: [0.0, 1.0, 0.0],
                    ..RigidBody3d::default()
                },
                &[Collider3d::cuboid([0.2, 0.3, 0.4])],
            )
            .unwrap();
        near3(
            world.angular_velocity(id(1)).unwrap(),
            if locked { [0.0; 3] } else { [0.0, 1.0, 0.0] },
        );
        world.set_angular_velocity(id(1), [0.0, 1.0, 0.0]).unwrap();
        world.step(Duration::from_millis(100)).unwrap();
        let rotation = world.pose(id(1)).unwrap().rotation;
        if locked {
            near3(world.angular_velocity(id(1)).unwrap(), [0.0; 3]);
            near(rotation[1], 0.0);
        } else {
            assert!(rotation[1] > 0.04);
        }
        near(rotation.into_iter().map(|part| part * part).sum(), 1.0);
    }
}

#[test]
fn invalid_body_or_late_piece_does_not_partially_register() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let invalid = RigidBody3d {
        rotation: [0.0; 4],
        ..RigidBody3d::default()
    };
    assert_eq!(
        world.insert_body(id(1), invalid, &[Collider3d::sphere(1.0)]),
        Err(PhysicsError::InvalidQuaternion("rotation"))
    );
    let pieces = [
        Collider3d::sphere(1.0),
        Collider3d::cuboid([1.0, -1.0, 1.0]),
    ];
    assert!(matches!(
        world.insert_body(id(1), RigidBody3d::default(), &pieces),
        Err(PhysicsError::ColliderPiece { index: 1, .. })
    ));
    assert!(world.is_empty());
    assert_eq!(
        world.insert_body(id(1), RigidBody3d::default(), &[]),
        Err(PhysicsError::NoColliderPieces(id(1)))
    );
    world
        .insert_body(id(1), RigidBody3d::default(), &[Collider3d::sphere(1.0)])
        .unwrap();
    assert_eq!(world.len(), 1);
    assert_eq!(
        world.insert_body(id(1), RigidBody3d::default(), &[Collider3d::sphere(1.0)]),
        Err(PhysicsError::EntityAlreadyRegistered(id(1)))
    );
}

#[test]
fn bad_controls_and_timesteps_leave_live_state_unchanged() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_static_collider(id(1), at([1.0, 2.0, 3.0]), &[Collider3d::sphere(1.0)])
        .unwrap();
    let before = world.pose(id(1)).unwrap();
    assert!(world.set_linear_velocity(id(1), [1.0; 3]).is_err());
    assert!(world.set_angular_velocity(id(1), [1.0; 3]).is_err());
    assert!(world.apply_impulse(id(1), [1.0; 3]).is_err());
    assert!(world.set_kinematic_target(id(1), at([4.0; 3])).is_err());
    assert!(
        world
            .move_to(
                id(1),
                PhysicsPose3d {
                    rotation: [f32::NAN; 4],
                    ..before
                }
            )
            .is_err()
    );
    assert_eq!(
        world.step(Duration::ZERO),
        Err(PhysicsError::InvalidTimestep)
    );
    assert!(world.set_gravity([f32::INFINITY; 3]).is_err());
    assert_eq!(world.pose(id(1)).unwrap(), before);
    near3(world.gravity(), [0.0; 3]);
}

#[test]
fn removal_and_slot_reuse_discard_old_handles_and_events() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_static_collider(
            id(1),
            at([0.0; 3]),
            &[Collider3d {
                sensor: true,
                ..Collider3d::sphere(1.0)
            }],
        )
        .unwrap();
    world
        .insert_body(id(2), RigidBody3d::default(), &[Collider3d::sphere(0.5)])
        .unwrap();
    world.step(STEP).unwrap();
    assert!(world.remove(id(2)));
    assert!(!world.remove(id(2)));
    assert_eq!(world.pose(id(2)), Err(PhysicsError::MissingEntity(id(2))));
    world
        .insert_body(id(3), RigidBody3d::default(), &[Collider3d::sphere(0.5)])
        .unwrap();
    let events = world.step(STEP).unwrap();
    assert!(
        events
            .iter()
            .all(|event| event.first != id(2) && event.second != id(2))
    );
    assert!(
        events
            .iter()
            .any(|event| event.first == id(1) && event.second == id(3))
    );
    assert!(world.contains(id(3)));
    world.remove(id(3));
    world.remove(id(1));
    assert!(world.is_empty());
}

#[test]
fn near_unit_rotations_are_normalized_but_nonunit_values_fail_atomically() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_body(
            id(1),
            RigidBody3d {
                rotation: [0.0, 0.0, 0.0, 1.00004],
                ..RigidBody3d::default()
            },
            &[Collider3d::sphere(0.5)],
        )
        .unwrap();
    near(world.pose(id(1)).unwrap().rotation[3], 1.0);
    let before = world.pose(id(1)).unwrap();
    for rotation in [[0.0; 4], [0.0, 0.0, 0.0, 2.0], [f32::MAX; 4]] {
        assert_eq!(
            world.move_to(id(1), PhysicsPose3d { rotation, ..before }),
            Err(PhysicsError::InvalidQuaternion("rotation"))
        );
        assert_eq!(world.pose(id(1)).unwrap(), before);
    }
}
