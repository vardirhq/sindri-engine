//! Solver snapshots describe support and actual impulses without backend handles.

use std::time::Duration;

use sindri_core::EntityId;
use sindri_physics::{Collider2d, PhysicsPose2d, PhysicsWorld2d, RigidBody2d};

const STEP: Duration = Duration::from_nanos(16_666_667);

fn entity(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}

fn supported(pieces: &[Collider2d]) -> PhysicsWorld2d {
    let mut world = PhysicsWorld2d::new([0.0, -10.0]).unwrap();
    world
        .insert_static_collider(
            entity(2),
            PhysicsPose2d::default(),
            &[Collider2d::rectangle([4.0, 0.5])],
        )
        .unwrap();
    world
        .insert_body(
            entity(1),
            RigidBody2d {
                pose: PhysicsPose2d {
                    position: [0.0, 1.0],
                    rotation: 0.0,
                },
                lock_rotation: true,
                ..RigidBody2d::default()
            },
            pieces,
        )
        .unwrap();
    world
}

fn near(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() < tolerance,
        "{actual} != {expected}"
    );
}

#[test]
fn support_points_normals_and_solved_force_match_momentum() {
    let mut world = supported(&[Collider2d::rectangle([0.5; 2])]);
    for _ in 0..20 {
        world.step(STEP).unwrap();
    }
    world.set_linear_velocity(entity(1), [2.0, -3.0]).unwrap();
    let before = world.linear_velocity(entity(1)).unwrap();
    world.step(STEP).unwrap();
    let after = world.linear_velocity(entity(1)).unwrap();
    let contacts = world.contacts(entity(1)).unwrap();
    assert!(!contacts.is_empty());
    let opposite = world.contacts(entity(2)).unwrap();
    let dt = STEP.as_secs_f32();
    let mass = world.mass(entity(1)).unwrap();
    let mut force = [0.0; 2];
    for (contact, other) in contacts.iter().zip(&opposite) {
        assert_eq!(contact.entity, entity(2));
        assert_eq!(other.entity, entity(1));
        near(contact.point[1], 0.5, 0.03);
        assert!(contact.normal[1] > 0.99);
        assert!(other.normal[1] < -0.99);
        assert!(contact.normal_impulse > 0.0);
        near(contact.force[0], -other.force[0], 1.0e-5);
        near(contact.force[1], -other.force[1], 1.0e-5);
        near(contact.force[0], -contact.tangent_impulse / dt, 1.0e-4);
        force[0] += contact.force[0];
        force[1] += contact.force[1];
    }
    // Subtract the separately applied gravity impulse from the velocity change.
    near(force[0] * dt, mass * (after[0] - before[0]), 0.02);
    near(
        force[1] * dt,
        mass * (after[1] - before[1] + 10.0 * dt),
        0.02,
    );
    assert!(force[0] < 0.0, "friction opposes the rightward motion");
}

#[test]
fn compound_pieces_do_not_lose_or_double_count_impulses() {
    let pieces = [
        Collider2d {
            offset: [-0.3, 0.0],
            ..Collider2d::circle(0.5)
        },
        Collider2d {
            offset: [0.3, 0.0],
            ..Collider2d::circle(0.5)
        },
    ];
    let mut world = supported(&pieces);
    for _ in 0..30 {
        world.step(STEP).unwrap();
    }
    let contacts = world.contacts(entity(1)).unwrap();
    assert!(
        contacts.len() >= 2,
        "both pieces support the body: {contacts:?}"
    );
    let total: f32 = contacts.iter().map(|contact| contact.force[1]).sum();
    near(total, world.mass(entity(1)).unwrap() * 10.0, 0.2);
    assert!(
        contacts
            .windows(2)
            .all(|pair| pair[0].point[0] <= pair[1].point[0])
    );
}

#[test]
fn contacts_are_copies_and_removal_teleport_and_invalid_steps_are_safe() {
    let mut world = supported(&[Collider2d::rectangle([0.5; 2])]);
    assert!(world.contacts(entity(1)).unwrap().is_empty());
    assert!(world.contacts(entity(99)).is_err());
    world.step(STEP).unwrap();
    let snapshot = world.contacts(entity(1)).unwrap();
    assert!(!snapshot.is_empty());
    assert!(world.step(Duration::ZERO).is_err());
    assert_eq!(snapshot, world.contacts(entity(1)).unwrap());
    let mut edited = snapshot.clone();
    edited[0].point = [99.0; 2];
    assert_eq!(snapshot, world.contacts(entity(1)).unwrap());
    world
        .move_to(
            entity(1),
            PhysicsPose2d {
                position: [0.0, 8.0],
                rotation: 0.0,
            },
        )
        .unwrap();
    assert!(world.contacts(entity(1)).unwrap().is_empty());
    assert!(world.contacts(entity(2)).unwrap().is_empty());
    world
        .move_to(
            entity(1),
            PhysicsPose2d {
                position: [0.0, 1.0],
                rotation: 0.0,
            },
        )
        .unwrap();
    world.step(STEP).unwrap();
    assert!(!world.contacts(entity(1)).unwrap().is_empty());
    world.remove(entity(2));
    assert!(world.contacts(entity(1)).unwrap().is_empty());
    assert!(!snapshot.is_empty(), "held copies outlive removal");
}

#[test]
fn sensors_never_supply_contacts_and_sleeping_support_has_no_new_impulse() {
    let mut world = supported(&[Collider2d::rectangle([0.5; 2])]);
    world
        .insert_static_collider(
            entity(3),
            PhysicsPose2d {
                position: [0.0, 1.0],
                rotation: 0.0,
            },
            &[Collider2d {
                sensor: true,
                ..Collider2d::rectangle([1.0; 2])
            }],
        )
        .unwrap();
    let mut saw_sensor_event = false;
    for _ in 0..600 {
        saw_sensor_event |= !world.step(STEP).unwrap().is_empty();
    }
    assert!(saw_sensor_event);
    assert!(world.contacts(entity(3)).unwrap().is_empty());
    let contacts = world.contacts(entity(1)).unwrap();
    assert!(!contacts.is_empty(), "sleeping body retains support");
    for contact in contacts {
        assert_eq!(contact.entity, entity(2));
        assert!(contact.normal[1] > 0.99);
        near(contact.normal_impulse, 0.0, 1.0e-6);
        near(contact.tangent_impulse, 0.0, 1.0e-6);
        near(contact.force[1], 0.0, 1.0e-6);
    }
}

#[test]
fn repeated_runs_order_multiple_other_entities_identically() {
    fn run(reverse: bool) -> Vec<sindri_physics::Contact2d> {
        let mut world = PhysicsWorld2d::new([0.0, -10.0]).unwrap();
        let order = if reverse { [3, 2] } else { [2, 3] };
        for index in order {
            world
                .insert_static_collider(
                    entity(index),
                    PhysicsPose2d {
                        position: [if index == 2 { -0.5 } else { 0.5 }, 0.0],
                        rotation: 0.0,
                    },
                    &[Collider2d::rectangle([0.49, 0.5])],
                )
                .unwrap();
        }
        world
            .insert_body(
                entity(1),
                RigidBody2d {
                    pose: PhysicsPose2d {
                        position: [0.0, 1.0],
                        rotation: 0.0,
                    },
                    lock_rotation: true,
                    ..RigidBody2d::default()
                },
                &[Collider2d::rectangle([1.0, 0.5])],
            )
            .unwrap();
        world.step(STEP).unwrap();
        world.contacts(entity(1)).unwrap()
    }
    for reverse in [false, true] {
        let a = run(reverse);
        assert_eq!(a, run(reverse));
        assert!(a.iter().any(|c| c.entity == entity(2)));
        assert!(a.iter().any(|c| c.entity == entity(3)));
        assert!(a.windows(2).all(|pair| pair[0].entity <= pair[1].entity));
    }
}

#[test]
fn rotated_support_reports_world_normals_and_points() {
    let angle = std::f32::consts::FRAC_PI_6;
    let normal = [-angle.sin(), angle.cos()];
    let mut world = PhysicsWorld2d::new(normal.map(|n| -n * 10.0)).unwrap();
    world
        .insert_static_collider(
            entity(2),
            PhysicsPose2d {
                position: [0.0; 2],
                rotation: angle,
            },
            &[Collider2d::rectangle([4.0, 0.5])],
        )
        .unwrap();
    world
        .insert_body(
            entity(1),
            RigidBody2d {
                pose: PhysicsPose2d {
                    position: normal,
                    rotation: 0.0,
                },
                ..RigidBody2d::default()
            },
            &[Collider2d::circle(0.5)],
        )
        .unwrap();
    world.step(STEP).unwrap();
    let contacts = world.contacts(entity(1)).unwrap();
    assert!(!contacts.is_empty());
    for contact in contacts {
        near(contact.normal[0], normal[0], 0.01);
        near(contact.normal[1], normal[1], 0.01);
        near(
            contact.point[0] * normal[0] + contact.point[1] * normal[1],
            0.5,
            0.03,
        );
    }
}
