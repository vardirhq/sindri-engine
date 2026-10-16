//! Ground classification independent of input, gravity and platform carry.
use crate::{CharacterOptions3d, PhysicsError, ShapeHit3d, validate::non_negative};

/// Settings for a read-only downward support query, in world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundOptions3d {
    /// Unit world-space up, checked within 0.0001 of squared unit length.
    pub up: [f32; 3],
    /// Maximum walkable deviation from up, in radians within 0..=pi/2.
    pub max_slope_angle: f32,
    /// Maximum downward travel to the skin, not to physical contact.
    pub max_distance: f32,
    /// Positive finite separation from the surface.
    pub skin: f32,
}

impl Default for GroundOptions3d {
    fn default() -> Self {
        Self {
            up: [0.0, 1.0, 0.0],
            max_slope_angle: std::f32::consts::FRAC_PI_4,
            max_distance: 0.1,
            skin: 0.01,
        }
    }
}

impl GroundOptions3d {
    /// # Errors
    /// Rejects invalid up, slope angle, travel or skin.
    pub fn validate(self) -> Result<(), PhysicsError> {
        CharacterOptions3d {
            up: self.up,
            max_slope_angle: self.max_slope_angle,
            skin: self.skin,
            ..CharacterOptions3d::default()
        }
        .validate()?;
        non_negative("ground_max_distance", self.max_distance)
    }
}

/// Nearest downward surface; classification never probes through steep support.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundProbe3d {
    /// World geometry and travel to the skin; zero for existing skin contact.
    pub hit: Option<ShapeHit3d>,
    /// An upward-facing normal within the configured slope limit.
    /// This classifies a surface, not whether gameplay is standing on it.
    pub walkable: bool,
    /// Initial overlap blocks support and returns a zero-normal hit.
    pub started_penetrating: bool,
}
