//! Sindri-owned 3D body, collider, pose and event values.

use serde::{Deserialize, Serialize};

use sindri_core::EntityId;

use crate::PhysicsError;
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

/// A hard maximum-distance constraint between anchors on two 3D bodies: a rope.
///
/// The anchors may come closer and the bodies turn freely, but the solver will
/// not let the anchors separate beyond `max_distance`. Anchors are body-local
/// world units without transform scale; zero is the body's origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DistanceJoint3d {
    pub first: EntityId,
    pub second: EntityId,
    pub max_distance: f32,
    pub first_anchor: [f32; 3],
    pub second_anchor: [f32; 3],
}

impl DistanceJoint3d {
    /// A rope between the two bodies' origins.
    pub const fn new(first: EntityId, second: EntityId, max_distance: f32) -> Self {
        Self {
            first,
            second,
            max_distance,
            first_anchor: [0.0; 3],
            second_anchor: [0.0; 3],
        }
    }

    /// Checks the joint without needing a world.
    ///
    /// # Errors
    /// Identical endpoints, a non-positive or non-finite distance, or a
    /// non-finite anchor.
    pub fn validate(&self) -> Result<(), PhysicsError> {
        if self.first == self.second {
            return Err(PhysicsError::JointToSelf(self.first));
        }
        crate::validate::positive("distance_joint_max_distance", self.max_distance)?;
        crate::world3d::finite3("distance_joint_first_anchor", self.first_anchor)?;
        crate::world3d::finite3("distance_joint_second_anchor", self.second_anchor)
    }
}
