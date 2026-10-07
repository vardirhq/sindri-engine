//! Authored 3D physics values and dimension ownership validation.

use serde::Deserialize;
use sindri_core::{SceneComponent, World};
use sindri_physics::{Collider3d, RigidBody3d};

use crate::PhysicsSyncError;

/// Authored body settings. The entity transform supplies its initial world pose.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct RigidBody3dComponent(pub RigidBody3d);

impl SceneComponent for RigidBody3dComponent {
    const TYPE_NAME: &'static str = "sindri.physics3d.rigid_body";
}

/// One shape or a compound of local box/sphere/Y-capsule pieces.
/// Dimensions and offsets are in world units, independent of visual scale.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(from = "AuthoredCollider3d")]
pub struct Collider3dComponent(pub Vec<Collider3d>);

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum AuthoredCollider3d {
    Compound { pieces: Vec<Collider3d> },
    Single(Collider3d),
}

impl From<AuthoredCollider3d> for Collider3dComponent {
    fn from(authored: AuthoredCollider3d) -> Self {
        Self(match authored {
            AuthoredCollider3d::Compound { pieces } => pieces,
            AuthoredCollider3d::Single(piece) => vec![piece],
        })
    }
}

impl SceneComponent for Collider3dComponent {
    const TYPE_NAME: &'static str = "sindri.physics3d.collider";
}

/// Scene gravity overrides the host while this component is active.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PhysicsWorld3dComponent {
    #[serde(default = "earth_gravity")]
    pub gravity: [f32; 3],
    /// Layer labels in bit order for `Physics3d.layer` and `Physics3d.mask`.
    #[serde(default)]
    pub layers: Vec<String>,
}

const fn earth_gravity() -> [f32; 3] {
    [0.0, -9.81, 0.0]
}

impl SceneComponent for PhysicsWorld3dComponent {
    const TYPE_NAME: &'static str = "sindri.physics3d.world";
}

pub(crate) fn validate_dimensions(world: &World) -> Result<(), PhysicsSyncError> {
    for (entity, data) in world
        .entities()
        .filter(|(entity, _)| world.is_active(*entity))
    {
        let has_3d = [
            RigidBody3dComponent::TYPE_NAME,
            Collider3dComponent::TYPE_NAME,
        ]
        .into_iter()
        .any(|name| data.components.contains_key(name));
        let has_2d = [
            crate::RigidBody2dComponent::TYPE_NAME,
            crate::Collider2dComponent::TYPE_NAME,
            crate::TilemapCollider2dComponent::TYPE_NAME,
            crate::Character2dComponent::TYPE_NAME,
        ]
        .into_iter()
        .any(|name| data.components.contains_key(name));
        if has_3d && has_2d {
            return Err(PhysicsSyncError::ConflictingDimensions(entity));
        }
    }
    Ok(())
}
