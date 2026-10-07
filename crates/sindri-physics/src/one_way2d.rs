//! Validated, engine-owned support-side collision policy.

use serde::{Deserialize, Serialize};

use crate::PhysicsError;
use crate::validate::{finite2, non_negative, positive};

/// Solid pieces support bodies only on the side pointed to by this local normal.
/// Sensors and geometric queries retain their ordinary behavior.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "AuthoredOneWay2d")]
pub struct OneWay2d {
    pub normal: [f32; 2],
    /// Maximum deviation of a supporting contact normal, in radians (0..=pi/2).
    pub angle: f32,
}

impl Default for OneWay2d {
    fn default() -> Self {
        Self {
            normal: [0.0, 1.0],
            angle: std::f32::consts::FRAC_PI_4,
        }
    }
}

impl OneWay2d {
    pub fn validate(self) -> Result<(), PhysicsError> {
        finite2("one_way_normal", self.normal)?;
        positive(
            "one_way_normal_length",
            self.normal[0].hypot(self.normal[1]),
        )?;
        non_negative("one_way_angle", self.angle)?;
        if self.angle > std::f32::consts::FRAC_PI_2 {
            return Err(PhysicsError::InvalidOneWayAngle);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct AuthoredOneWay2d {
    normal: [f32; 2],
    angle: f32,
}

impl Default for AuthoredOneWay2d {
    fn default() -> Self {
        let policy = OneWay2d::default();
        Self {
            normal: policy.normal,
            angle: policy.angle,
        }
    }
}

impl TryFrom<AuthoredOneWay2d> for OneWay2d {
    type Error = PhysicsError;

    fn try_from(value: AuthoredOneWay2d) -> Result<Self, Self::Error> {
        let policy = Self {
            normal: value.normal,
            angle: value.angle,
        };
        policy.validate()?;
        Ok(policy)
    }
}
