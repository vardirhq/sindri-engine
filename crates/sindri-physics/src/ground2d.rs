//! Ground classification independent of input, gravity and jump policy.

use crate::validate::{finite2, non_negative, positive};
use crate::{PhysicsError, ShapeHit2d};

/// Settings for a read-only downward support query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundOptions2d {
    /// Unit world-space up direction; checked within 0.0001 of unit length.
    pub up: [f32; 2],
    /// Maximum walkable deviation from up, in radians, within 0..=pi/2.
    pub max_slope_angle: f32,
    /// Maximum downward travel before reaching the skin, in world units.
    pub max_distance: f32,
    /// Positive finite surface separation, in world units.
    pub skin: f32,
}

impl Default for GroundOptions2d {
    fn default() -> Self {
        Self {
            up: [0.0, 1.0],
            max_slope_angle: std::f32::consts::FRAC_PI_4,
            max_distance: 0.1,
            skin: 0.01,
        }
    }
}

impl GroundOptions2d {
    /// # Errors
    /// Rejects invalid up, slope angle, travel distance or skin.
    pub fn validate(self) -> Result<(), PhysicsError> {
        finite2("ground_up", self.up)?;
        let length = self.up[0].hypot(self.up[1]);
        if !length.is_finite() || (length - 1.0).abs() > 0.0001 {
            return Err(PhysicsError::InvalidGroundUp);
        }
        non_negative("ground_max_slope_angle", self.max_slope_angle)?;
        if self.max_slope_angle > std::f32::consts::FRAC_PI_2 {
            return Err(PhysicsError::InvalidGroundSlopeAngle);
        }
        non_negative("ground_max_distance", self.max_distance)?;
        positive("ground_skin", self.skin)
    }
}

/// Nearest downward contact; classification never skips an obstructing surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundProbe2d {
    /// The nearest surface, including steep surfaces and initial penetration.
    /// Its distance is travel to the skin, rather than the unpadded surface.
    pub hit: Option<ShapeHit2d>,
    /// A hit normal within the configured slope angle; false without a hit or
    /// during initial penetration. This does not mean the caller is standing on it.
    pub walkable: bool,
    /// Initial penetration blocks the query and returns a zero-normal hit.
    pub started_penetrating: bool,
}
