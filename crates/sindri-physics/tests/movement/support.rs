//! Shared fixtures for geometric movement and ground probing.
use sindri_core::EntityId;
use sindri_physics::{Collider2d, PhysicsPose2d, PhysicsWorld2d};

pub fn entity(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}

pub fn pose(x: f32, y: f32) -> PhysicsPose2d {
    PhysicsPose2d {
        position: [x, y],
        rotation: 0.0,
    }
}

pub fn insert(world: &mut PhysicsWorld2d, id: u32, at: PhysicsPose2d, pieces: &[Collider2d]) {
    world
        .insert_static_collider(entity(id), at, pieces)
        .unwrap();
}

pub fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.002, "{actual} != {expected}");
}
