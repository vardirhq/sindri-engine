//! Support state and optional snapping composed with geometric sliding.

use crate::{GroundOptions2d, GroundProbe2d, PhysicsError, SlideMotion2d, SlideOptions2d};

/// Sweep/slide and support settings; gravity and jump decisions remain gameplay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundedSlideOptions2d {
    /// Sweep separation and iteration budget; also supplies the support skin.
    pub slide: SlideOptions2d,
    /// Unit world-space up direction, checked within 0.0001 of unit length.
    pub up: [f32; 2],
    /// Maximum walkable deviation from up, in radians, within 0..=pi/2.
    pub max_slope_angle: f32,
    /// Optional downward travel to retain support. Zero disables snapping.
    /// Gameplay can enable this while previously grounded and disable it in air.
    /// Requested upward movement always suppresses snapping.
    pub snap_distance: f32,
}

impl Default for GroundedSlideOptions2d {
    fn default() -> Self {
        Self {
            slide: SlideOptions2d::default(),
            up: [0.0, 1.0],
            max_slope_angle: std::f32::consts::FRAC_PI_4,
            snap_distance: 0.0,
        }
    }
}

impl GroundedSlideOptions2d {
    /// # Errors
    /// Rejects invalid slide settings, up, slope angle or snap distance.
    pub fn validate(self) -> Result<(), PhysicsError> {
        self.slide.validate()?;
        self.ground_options().validate()
    }

    pub(crate) fn ground_options(self) -> GroundOptions2d {
        GroundOptions2d {
            up: self.up,
            max_slope_angle: self.max_slope_angle,
            max_distance: self.snap_distance,
            skin: self.slide.skin,
        }
    }
}

/// Read-only proposal combining collision-limited movement and final support.
#[derive(Clone, Debug, PartialEq)]
pub struct GroundedSlideMotion2d {
    /// Complete displacement, including any accepted downward snap.
    pub translation: [f32; 2],
    /// Unmodified geometric slide result; its translation excludes the snap.
    pub slide: SlideMotion2d,
    /// Downward support query at the slide endpoint, before any snap.
    /// Steep hits remain visible even though they never cause snapping.
    pub ground: GroundProbe2d,
    /// Additional downward displacement, zero when no snap is accepted.
    pub snap_translation: [f32; 2],
    /// Walkable support at the proposed endpoint, including accepted snapping.
    /// Initial penetration is never grounded. Upward requests are not grounded.
    pub grounded: bool,
}
