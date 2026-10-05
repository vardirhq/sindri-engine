//! A 2D hinge joins body-local anchors and permits relative rotation.

use serde::{Deserialize, Serialize};
use sindri_core::EntityId;

use crate::PhysicsError;
use crate::validate::{finite, finite2, non_negative};

/// Distances use world units in each body's translated/rotated local frame;
/// transform scale is not applied. Angles and speeds use radians.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HingeSettings2d {
    pub first_anchor: [f32; 2],
    pub second_anchor: [f32; 2],
    pub limits_enabled: bool,
    pub lower_angle: f32,
    pub upper_angle: f32,
    pub motor_enabled: bool,
    pub motor_velocity: f32,
    /// Maximum motor torque. Zero supplies no drive or braking torque.
    pub motor_max_torque: f32,
}

impl HingeSettings2d {
    /// Checks all values, including settings whose toggle is disabled.
    ///
    /// # Errors
    /// Rejects nonfinite values, negative torque and enabled limits outside
    /// `[-pi, pi]` or whose lower bound exceeds their upper bound.
    pub fn validate(self) -> Result<(), PhysicsError> {
        finite2("hinge_first_anchor", self.first_anchor)?;
        finite2("hinge_second_anchor", self.second_anchor)?;
        finite("hinge_lower_angle", self.lower_angle)?;
        finite("hinge_upper_angle", self.upper_angle)?;
        finite("hinge_motor_velocity", self.motor_velocity)?;
        non_negative("hinge_motor_max_torque", self.motor_max_torque)?;
        if self.limits_enabled
            && (self.lower_angle > self.upper_angle
                || self.lower_angle < -std::f32::consts::PI
                || self.upper_angle > std::f32::consts::PI)
        {
            return Err(PhysicsError::InvalidJointLimits);
        }
        Ok(())
    }
}

/// Runtime endpoints are generation-checked entities, never serialized handles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HingeJoint2d {
    pub first: EntityId,
    pub second: EntityId,
    pub settings: HingeSettings2d,
}
