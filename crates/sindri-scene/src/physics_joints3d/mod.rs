//! Authored 3D constraints, resolved after every 3D body has synchronized.
//!
//! The 3D counterpart of `physics_joints`: a separate owner entity holds the
//! constraint, so several can meet at one body; endpoints are stable scene IDs
//! or prefab paths; an endpoint that is missing, inactive or has no body
//! suspends the constraint until it does.
mod sync;

use serde::{Deserialize, Serialize};
use sindri_core::SceneComponent;
use sindri_physics::{DistanceJoint3d, PhysicsError};

pub(crate) use sync::SceneJoints3d;

/// A rope between anchors on two 3D bodies: they may come closer and turn
/// freely, but never separate beyond `max_distance`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct DistanceJoint3dComponent {
    #[serde(default)]
    pub first: String,
    #[serde(default)]
    pub second: String,
    /// Suspends this constraint while keeping its endpoints and settings.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_distance")]
    pub max_distance: f32,
    /// Body-local points the rope is tied to, in world units without scale.
    #[serde(default)]
    pub first_anchor: [f32; 3],
    #[serde(default)]
    pub second_anchor: [f32; 3],
}

const fn default_enabled() -> bool {
    true
}

const fn default_distance() -> f32 {
    1.0
}

impl Default for DistanceJoint3dComponent {
    fn default() -> Self {
        Self {
            first: String::new(),
            second: String::new(),
            enabled: true,
            max_distance: default_distance(),
            first_anchor: [0.0; 3],
            second_anchor: [0.0; 3],
        }
    }
}

impl SceneComponent for DistanceJoint3dComponent {
    const TYPE_NAME: &'static str = "sindri.physics3d.distance_joint";
}

impl DistanceJoint3dComponent {
    /// Validates the length and anchors, independently of the endpoints.
    ///
    /// # Errors
    /// A non-finite or non-positive length, or a non-finite anchor.
    pub fn validate(&self) -> Result<(), PhysicsError> {
        self.joint(
            sindri_core::EntityId::from_bits(1 << 32),
            sindri_core::EntityId::from_bits(2 << 32),
        )
        .validate()
    }

    /// The solver's joint between resolved endpoints.
    #[must_use]
    pub const fn joint(
        &self,
        first: sindri_core::EntityId,
        second: sindri_core::EntityId,
    ) -> DistanceJoint3d {
        DistanceJoint3d {
            first,
            second,
            max_distance: self.max_distance,
            first_anchor: self.first_anchor,
            second_anchor: self.second_anchor,
        }
    }
}

#[cfg(test)]
mod tests;
