//! Force-based spring and damping between freely rotating local anchors.

use serde::{Deserialize, Serialize};
use sindri_core::EntityId;

use crate::PhysicsError;
use crate::validate::{finite2, non_negative, positive};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpringSettings2d {
    pub first_anchor: [f32; 2],
    pub second_anchor: [f32; 2],
    pub rest_length: f32,
    pub stiffness: f32,
    pub damping: f32,
}

impl Default for SpringSettings2d {
    fn default() -> Self {
        Self {
            first_anchor: [0.0; 2],
            second_anchor: [0.0; 2],
            rest_length: 1.0,
            stiffness: 10.0,
            damping: 1.0,
        }
    }
}

impl SpringSettings2d {
    /// Checks anchors, positive rest length and non-negative force coefficients.
    ///
    /// # Errors
    /// Rejects nonfinite values, non-positive rest length or negative coefficients.
    pub fn validate(self) -> Result<(), PhysicsError> {
        finite2("spring_first_anchor", self.first_anchor)?;
        finite2("spring_second_anchor", self.second_anchor)?;
        positive("spring_rest_length", self.rest_length)?;
        non_negative("spring_stiffness", self.stiffness)?;
        non_negative("spring_damping", self.damping)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringJoint2d {
    pub first: EntityId,
    pub second: EntityId,
    pub settings: SpringSettings2d,
}
