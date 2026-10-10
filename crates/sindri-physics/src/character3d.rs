//! Engine-owned settings and copied results for 3D character movement.

use serde::{Deserialize, Serialize};

use crate::validate::{finite, non_negative, positive};
use crate::{PhysicsError, ShapeHit3d};

/// Geometry policy, in world units. Gameplay owns speed, gravity and jumping.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CharacterOptions3d {
    /// Unit world-space up, accepted within 0.0001 of unit length.
    pub up: [f32; 3],
    /// Positive separation from obstacles, independent of probe dimensions.
    pub skin: f32,
    pub slide: bool,
    /// Maximum climbable angle from up, in radians within 0..=pi/2.
    pub max_slope_angle: f32,
    /// Minimum angle that permits downhill sliding, within 0..=pi/2.
    pub min_slide_angle: f32,
    /// Optional downward snap; zero disables it. Upward motion suppresses snap.
    pub snap_distance: f32,
    /// Optional step lift; zero disables it.
    pub step_height: f32,
    /// Required free forward space at a step landing.
    pub step_min_width: f32,
    pub step_dynamic_bodies: bool,
}

impl Default for CharacterOptions3d {
    fn default() -> Self {
        Self {
            up: [0.0, 1.0, 0.0],
            skin: 0.01,
            slide: true,
            max_slope_angle: std::f32::consts::FRAC_PI_4,
            min_slide_angle: std::f32::consts::FRAC_PI_4,
            snap_distance: 0.0,
            step_height: 0.0,
            step_min_width: 0.1,
            step_dynamic_bodies: false,
        }
    }
}

impl CharacterOptions3d {
    /// # Errors
    /// Rejects non-unit up, invalid angles, skin and snap/step distances.
    pub fn validate(self) -> Result<(), PhysicsError> {
        for axis in self.up {
            finite("character_up", axis)?;
        }
        let norm = self
            .up
            .into_iter()
            .map(|part| f64::from(part).powi(2))
            .sum::<f64>();
        if (norm - 1.0).abs() > 0.0001 {
            return Err(PhysicsError::InvalidGroundUp);
        }
        positive("character_skin", self.skin)?;
        for angle in [self.max_slope_angle, self.min_slide_angle] {
            finite("character_slope_angle", angle)?;
            if !(0.0..=std::f32::consts::FRAC_PI_2).contains(&angle) {
                return Err(PhysicsError::InvalidGroundSlopeAngle);
            }
        }
        non_negative("character_snap_distance", self.snap_distance)?;
        non_negative("character_step_height", self.step_height)?;
        non_negative("character_step_min_width", self.step_min_width)
    }
}

/// One movement-phase hit; internal stair and snap probes are not events.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterCollision3d {
    /// World point/normal and travel distance from this sweep's start.
    pub hit: ShapeHit3d,
    pub translation_applied: [f32; 3],
    pub translation_remaining: [f32; 3],
}

/// A read-only displacement proposal. Applying it is a separate host operation.
#[derive(Clone, Debug, PartialEq)]
pub struct CharacterMotion3d {
    pub translation: [f32; 3],
    /// Rapier's nearby upward-facing contact status, not a walkable-slope test.
    pub grounded: bool,
    /// Rapier's slope-handling flag; may also be set while climbing a slope.
    pub sliding_down_slope: bool,
    pub collisions: Vec<CharacterCollision3d>,
}
