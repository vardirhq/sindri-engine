//! A copy of a world goes on exactly as the original would: what recording
//! a run and scrubbing back through it rely on.

use std::time::Duration;

use sindri_core::EntityId;
use sindri_physics::{Collider2d, PhysicsPose2d, PhysicsWorld2d, RigidBody2d};

const STEP: Duration = Duration::from_nanos(16_666_667);

fn entity(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}

/// A pile of boxes falling onto a floor and each other, rotating, so the
/// solver's warm-start and the islands carry real state from step to step.
fn pile() -> PhysicsWorld2d {
    let mut world = PhysicsWorld2d::new([0.0, -10.0]).unwrap();
    world
        .insert_static_collider(
            entity(100),
            PhysicsPose2d::default(),
            &[Collider2d::rectangle([10.0, 0.5])],
        )
        .unwrap();
    for index in 0..12_u16 {
        let column = f32::from(index % 4);
        let row = f32::from(index / 4);
        world
            .insert_body(
                entity(u32::from(index) + 1),
                RigidBody2d {
                    pose: PhysicsPose2d {
                        position: [column * 0.9 - 1.3 + row * 0.2, 1.5 + row * 1.1],
                        rotation: 0.1 * column,
                    },
                    ..RigidBody2d::default()
                },
                &[Collider2d::rectangle([0.4, 0.4])],
            )
            .unwrap();
    }
    world
}

fn poses(world: &PhysicsWorld2d) -> Vec<[u32; 3]> {
    (1..=12)
        .map(|index| {
            let pose = world.pose(entity(index)).expect("a body");
            [
                pose.position[0].to_bits(),
                pose.position[1].to_bits(),
                pose.rotation.to_bits(),
            ]
        })
        .collect()
}

#[test]
fn a_copy_taken_mid_fall_steps_exactly_as_the_original() {
    let mut original = pile();
    for _ in 0..45 {
        original.step(STEP).unwrap();
    }
    let mut copy = original.clone();
    for _ in 0..240 {
        original.step(STEP).unwrap();
        copy.step(STEP).unwrap();
        assert_eq!(poses(&original), poses(&copy));
    }
}
