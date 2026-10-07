//! Sindri-owned 3D body, collider, pose and event values.

use serde::{Deserialize, Serialize};

use crate::shared::{CollisionLayers, RigidBodyKind};

/// The 3D body model uses XYZ vectors and `[x, y, z, w]` unit quaternions.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RigidBody3d {
    pub kind: RigidBodyKind,
    pub position: [f32; 3],
    /// Quaternion in `[x, y, z, w]` order.
    pub rotation: [f32; 4],
    pub linear_velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
    pub gravity_scale: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub lock_rotation: bool,
}

impl Default for RigidBody3d {
    fn default() -> Self {
        Self {
            kind: RigidBodyKind::Dynamic,
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            linear_velocity: [0.0; 3],
            angular_velocity: [0.0; 3],
            gravity_scale: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            lock_rotation: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum ColliderShape3d {
    Box { half_extents: [f32; 3] },
    Sphere { radius: f32 },
    Capsule { half_height: f32, radius: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Collider3d {
    pub shape: ColliderShape3d,
    pub offset: [f32; 3],
    /// Quaternion in `[x, y, z, w]` order.
    pub rotation: [f32; 4],
    pub sensor: bool,
    pub layers: CollisionLayers,
    pub friction: f32,
    pub restitution: f32,
}

/// A world-space 3D position and unit quaternion in `[x, y, z, w]` order.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicsPose3d {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
}

impl Default for PhysicsPose3d {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicsEvent3d {
    pub first: sindri_core::EntityId,
    pub second: sindri_core::EntityId,
    pub kind: crate::PhysicsEventKind,
}

impl Collider3d {
    pub const fn sphere(radius: f32) -> Self {
        Self {
            shape: ColliderShape3d::Sphere { radius },
            offset: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            sensor: false,
            layers: CollisionLayers::ALL,
            friction: 0.5,
            restitution: 0.0,
        }
    }

    pub const fn cuboid(half_extents: [f32; 3]) -> Self {
        Self {
            shape: ColliderShape3d::Box { half_extents },
            ..Self::sphere(1.0)
        }
    }

    pub const fn capsule(half_height: f32, radius: f32) -> Self {
        Self {
            shape: ColliderShape3d::Capsule {
                half_height,
                radius,
            },
            ..Self::sphere(1.0)
        }
    }
}
