//! Support state and optional snapping composed with geometric sliding.

use crate::validate::{non_negative, validate_pose2d};
use crate::{
    GroundOptions2d, GroundProbe2d, PhysicsError, PlatformCarry2d, PlatformSupport2d,
    SlideMotion2d, SlideOptions2d,
};

/// Sweep/slide and support settings; gravity and jump decisions remain gameplay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundedSlideOptions2d {
    /// Sweep separation and iteration budget; also supplies the support skin.
    pub slide: SlideOptions2d,
    /// Unit world-space up direction, checked within 0.0001 of unit length.
    pub up: [f32; 2],
    /// Maximum walkable deviation from up, in radians, within 0..=pi/2.
    /// Steeper support cannot introduce upward sliding beyond requested rise.
    pub max_slope_angle: f32,
    /// Optional downward travel to retain support. Zero disables snapping.
    /// Gameplay can enable this while previously grounded and disable it in air.
    /// Requested upward movement always suppresses snapping.
    pub snap_distance: f32,
    /// Optional maximum step lift in world units; zero disables stepping.
    /// Requires starting support, clear headroom and a walkable landing.
    pub step_height: f32,
    /// Prior support snapshot, opt-in. The host must not also add its motion
    /// through parenting, velocity or the requested character displacement.
    pub platform_support: Option<PlatformSupport2d>,
    /// Ignore one-way solids throughout this request, including support/carry.
    /// The host owns duration/cancellation; ordinary solids and sensors retain
    /// their filtering. False respects each piece's support side and cone.
    pub drop_through: bool,
}

impl Default for GroundedSlideOptions2d {
    fn default() -> Self {
        Self {
            slide: SlideOptions2d::default(),
            up: [0.0, 1.0],
            max_slope_angle: std::f32::consts::FRAC_PI_4,
            snap_distance: 0.0,
            step_height: 0.0,
            platform_support: None,
            drop_through: false,
        }
    }
}

impl GroundedSlideOptions2d {
    /// # Errors
    /// Rejects invalid slide settings, up, slope angle, snap/step distances or support pose.
    pub fn validate(self) -> Result<(), PhysicsError> {
        self.slide.validate()?;
        self.ground_options().validate()?;
        non_negative("ground_step_height", self.step_height)?;
        if let Some(support) = self.platform_support {
            validate_pose2d(support.previous_pose)?;
        }
        Ok(())
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
    /// Complete displacement, including carry, step lift and downward landing/snap.
    pub translation: [f32; 2],
    /// Slope-limited slide result; excludes carry, step lift and landing/snap.
    /// Starts after carry, or at the lifted pose for an accepted step.
    pub slide: SlideMotion2d,
    /// Downward support query at the slide endpoint, before landing/snap.
    /// Steep hits remain visible even though they never cause snapping.
    pub ground: GroundProbe2d,
    /// Additional downward landing/snap displacement; zero without either.
    /// An accepted step can land even when ordinary snapping is disabled.
    pub snap_translation: [f32; 2],
    /// Accepted step lift, zero when ordinary sliding was chosen.
    /// Add this, slide, snap and platform motion translations to obtain total motion.
    pub step_translation: [f32; 2],
    /// Verified support carry, applied before the selected slide/step path.
    /// Its actual translation is also included in total translation.
    pub platform: Option<PlatformCarry2d>,
    /// Walkable support at the proposed endpoint, including accepted snapping.
    /// Initial penetration is never grounded. Upward requests are not grounded.
    pub grounded: bool,
}
