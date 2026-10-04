//! Sindri-owned physics types and runtime adapters.
//!
//! Rapier is deliberately private implementation detail. Scenes, the editor,
//! Decay, and games speak only in the types defined here. The first runtime
//! slice is 2D; the parallel 3D data model exists so the 2D API cannot quietly
//! become the dimension-neutral contract.

mod contact2d;
mod one_way2d;
mod query2d;
mod shared;
mod types2d;
mod types3d;
mod validate;
mod world2d;

#[cfg(test)]
mod tests;

pub use contact2d::Contact2d;
pub use one_way2d::OneWay2d;
pub use query2d::{RayHit2d, RaycastFilter2d};
pub use shared::{CollisionLayers, RigidBodyKind};
pub use types2d::{
    Collider2d, ColliderShape2d, DistanceJoint2d, PhysicsEvent2d, PhysicsEventKind, PhysicsPose2d,
    RigidBody2d,
};
pub use types3d::{Collider3d, ColliderShape3d, RigidBody3d};
pub use validate::PhysicsError;
pub use world2d::{BodyControl2d, PhysicsWorld2d, ShapeHit2d};
