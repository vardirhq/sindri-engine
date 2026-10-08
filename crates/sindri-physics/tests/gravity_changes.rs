//! Changing gravity reaches bodies that have fallen asleep at rest.

use std::time::Duration;

use sindri_core::EntityId;
use sindri_physics::{Collider2d, PhysicsPose2d, PhysicsWorld2d, RigidBody2d};

const STEP: Duration = Duration::from_nanos(16_666_667);

fn entity(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}

#[test]
fn a_box_asleep_on_the_floor_falls_up_when_gravity_flips() {
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
                ..RigidBody2d::default()
            },
            &[Collider2d::rectangle([0.5; 2])],
        )
        .unwrap();
    // Long enough for the solver to put the resting box to sleep.
    for _ in 0..600 {
        world.step(STEP).unwrap();
    }
    let resting = world.pose(entity(1)).unwrap().position[1];
    world.set_gravity([0.0, 10.0]).unwrap();
    for _ in 0..30 {
        world.step(STEP).unwrap();
    }
    let lifted = world.pose(entity(1)).unwrap().position[1];
    assert!(lifted > resting + 1.0, "{resting} -> {lifted}");
}

#[test]
fn a_3d_crate_asleep_on_the_floor_falls_up_when_gravity_flips() {
    use sindri_physics::{Collider3d, PhysicsPose3d, PhysicsWorld3d, RigidBody3d};
    let mut world = PhysicsWorld3d::new([0.0, -10.0, 0.0]).unwrap();
    world
        .insert_static_collider(
            entity(2),
            PhysicsPose3d::default(),
            &[Collider3d::cuboid([4.0, 0.5, 4.0])],
        )
        .unwrap();
    world
        .insert_body(
            entity(1),
            RigidBody3d {
                position: [0.0, 1.0, 0.0],
                ..RigidBody3d::default()
            },
            &[Collider3d::cuboid([0.5; 3])],
        )
        .unwrap();
    for _ in 0..600 {
        world.step(STEP).unwrap();
    }
    let resting = world.pose(entity(1)).unwrap().position[1];
    world.set_gravity([0.0, 10.0, 0.0]).unwrap();
    for _ in 0..30 {
        world.step(STEP).unwrap();
    }
    let lifted = world.pose(entity(1)).unwrap().position[1];
    assert!(lifted > resting + 1.0, "{resting} -> {lifted}");
}
