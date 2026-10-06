//! Sindri-owned physics types and runtime adapters.
//!
//! Rapier is deliberately private implementation detail. Scenes, the editor,
//! Decay, and games speak only in the types defined here. The first runtime
//! slice is 2D; the parallel 3D data model exists so the 2D API cannot quietly
//! become the dimension-neutral contract.

mod contact2d;
mod ground2d;
mod grounded2d;
mod hinge2d;
mod material;
mod motor2d;
mod one_way2d;
mod query2d;
mod shared;
mod slide2d;
mod slider2d;
mod spring2d;
mod types2d;
mod types3d;
mod validate;
mod world2d;

#[cfg(test)]
mod tests;

pub use contact2d::Contact2d;
pub use ground2d::{GroundOptions2d, GroundProbe2d};
pub use grounded2d::{GroundedSlideMotion2d, GroundedSlideOptions2d};
pub use hinge2d::{HingeJoint2d, HingeSettings2d};
pub use material::PhysicsMaterial;
pub use motor2d::MotorMode2d;
pub use one_way2d::OneWay2d;
pub use query2d::{RayHit2d, RaycastFilter2d};
pub use shared::{CollisionLayers, RigidBodyKind};
pub use slide2d::{SlideMotion2d, SlideOptions2d};
pub use slider2d::{SliderJoint2d, SliderSettings2d};
pub use spring2d::{SpringJoint2d, SpringSettings2d};
pub use types2d::{
    Collider2d, ColliderShape2d, DistanceJoint2d, PhysicsEvent2d, PhysicsEventKind, PhysicsPose2d,
    RigidBody2d,
};
pub use types3d::{Collider3d, ColliderShape3d, RigidBody3d};
pub use validate::PhysicsError;
pub use world2d::{BodyControl2d, PhysicsWorld2d, ShapeHit2d};
