//! Translation along aligned body-local axes, with rotation constrained.

use serde::{Deserialize, Serialize};
use sindri_core::EntityId;

use crate::validate::{finite, finite2, non_negative};
use crate::{MotorMode2d, PhysicsError};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SliderSettings2d {
    pub first_anchor: [f32; 2],
    pub second_anchor: [f32; 2],
    pub first_axis: [f32; 2],
    pub second_axis: [f32; 2],
    pub limits_enabled: bool,
    pub lower_distance: f32,
    pub upper_distance: f32,
    pub motor_enabled: bool,
    pub motor_mode: MotorMode2d,
    /// Signed anchor separation along the first body's local axis, in world units.
    pub motor_target_distance: f32,
    /// Force per world unit of position error.
    pub motor_stiffness: f32,
    /// Damping force per world unit per second.
    pub motor_damping: f32,
    /// Relative translation speed in world units per second.
    pub motor_velocity: f32,
    pub motor_max_force: f32,
}

impl Default for SliderSettings2d {
    fn default() -> Self {
        Self {
            first_anchor: [0.0; 2],
            second_anchor: [0.0; 2],
            first_axis: [1.0, 0.0],
            second_axis: [1.0, 0.0],
            limits_enabled: false,
            lower_distance: 0.0,
            upper_distance: 0.0,
            motor_enabled: false,
            motor_mode: MotorMode2d::Velocity,
            motor_target_distance: 0.0,
            motor_stiffness: 0.0,
            motor_damping: 0.0,
            motor_velocity: 0.0,
            motor_max_force: 0.0,
        }
    }
}

impl SliderSettings2d {
    /// Validates all numeric fields, including disabled settings.
    ///
    /// # Errors
    /// Rejects nonfinite values, non-unit axes, negative force caps/gains and
    /// enabled limits whose lower bound exceeds the upper bound.
    pub fn validate(self) -> Result<(), PhysicsError> {
        finite2("slider_first_anchor", self.first_anchor)?;
        finite2("slider_second_anchor", self.second_anchor)?;
        for axis in [self.first_axis, self.second_axis] {
            finite2("slider_axis", axis)?;
            if (axis[0].hypot(axis[1]) - 1.0).abs() > 1e-4 {
                return Err(PhysicsError::InvalidJointAxis);
            }
        }
        finite("slider_lower_distance", self.lower_distance)?;
        finite("slider_upper_distance", self.upper_distance)?;
        finite("slider_motor_velocity", self.motor_velocity)?;
        finite("slider_motor_target_distance", self.motor_target_distance)?;
        non_negative("slider_motor_stiffness", self.motor_stiffness)?;
        non_negative("slider_motor_damping", self.motor_damping)?;
        non_negative("slider_motor_max_force", self.motor_max_force)?;
        if self.limits_enabled && self.lower_distance > self.upper_distance {
            return Err(PhysicsError::InvalidSliderLimits);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderJoint2d {
    pub first: EntityId,
    pub second: EntityId,
    pub settings: SliderSettings2d,
}
