//! A 2D hinge joins body-local anchors and permits relative rotation.

use serde::{Deserialize, Serialize};
use sindri_core::EntityId;

use crate::validate::{finite, finite2, non_negative};
use crate::{MotorMode2d, PhysicsError};

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
    pub motor_mode: MotorMode2d,
    /// Relative angular target in radians, within `[-pi, pi]` in position mode.
    pub motor_target_angle: f32,
    /// Force-based torque per radian of angular error.
    pub motor_stiffness: f32,
    /// Force-based damping torque per radian per second.
    pub motor_damping: f32,
    pub motor_velocity: f32,
    /// Maximum motor torque. Zero supplies no drive or braking torque.
    pub motor_max_torque: f32,
}

impl HingeSettings2d {
    /// Checks all values, including settings whose toggle is disabled.
    ///
    /// # Errors
    /// Rejects nonfinite values, negative torque/gains, position targets outside
    /// `[-pi, pi]`, and enabled limits outside
    /// `[-pi, pi]` or whose lower bound exceeds their upper bound.
    pub fn validate(self) -> Result<(), PhysicsError> {
        finite2("hinge_first_anchor", self.first_anchor)?;
        finite2("hinge_second_anchor", self.second_anchor)?;
        finite("hinge_lower_angle", self.lower_angle)?;
        finite("hinge_upper_angle", self.upper_angle)?;
        finite("hinge_motor_velocity", self.motor_velocity)?;
        finite("hinge_motor_target_angle", self.motor_target_angle)?;
        non_negative("hinge_motor_stiffness", self.motor_stiffness)?;
        non_negative("hinge_motor_damping", self.motor_damping)?;
        if self.motor_mode == MotorMode2d::Position
            && self.motor_target_angle.abs() > std::f32::consts::PI
        {
            return Err(PhysicsError::InvalidHingeMotorTarget);
        }
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
