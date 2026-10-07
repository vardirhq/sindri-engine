//! Authored controller settings reuse the entity's existing collider editor.
use serde::Deserialize;
use sindri_core::SceneComponent;
use sindri_physics::{GroundedSlideOptions2d, PhysicsError, SlideOptions2d};

/// Collision movement settings, with input, velocity and gravity owned by gameplay.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(try_from = "AuthoredCharacter2d")]
pub struct Character2dComponent {
    pub skin: f32,
    pub max_iterations: u8,
    pub up: [f32; 2],
    pub max_slope_angle: f32,
    pub snap_distance: f32,
    pub step_height: f32,
    pub carry_platforms: bool,
}

impl Default for Character2dComponent {
    fn default() -> Self {
        let options = GroundedSlideOptions2d::default();
        Self {
            skin: options.slide.skin,
            max_iterations: options.slide.max_iterations,
            up: options.up,
            max_slope_angle: options.max_slope_angle,
            snap_distance: options.snap_distance,
            step_height: options.step_height,
            carry_platforms: true,
        }
    }
}

impl Character2dComponent {
    pub(super) fn options(self) -> GroundedSlideOptions2d {
        GroundedSlideOptions2d {
            slide: SlideOptions2d {
                skin: self.skin,
                max_iterations: self.max_iterations,
            },
            up: self.up,
            max_slope_angle: self.max_slope_angle,
            snap_distance: self.snap_distance,
            step_height: self.step_height,
            ..GroundedSlideOptions2d::default()
        }
    }
}

impl SceneComponent for Character2dComponent {
    const TYPE_NAME: &'static str = "sindri.physics2d.character";
}

#[derive(Deserialize)]
#[serde(default)]
struct AuthoredCharacter2d {
    skin: f32,
    max_iterations: u8,
    up: [f32; 2],
    max_slope_angle: f32,
    snap_distance: f32,
    step_height: f32,
    carry_platforms: bool,
}

impl Default for AuthoredCharacter2d {
    fn default() -> Self {
        let defaults = Character2dComponent::default();
        Self {
            skin: defaults.skin,
            max_iterations: defaults.max_iterations,
            up: defaults.up,
            max_slope_angle: defaults.max_slope_angle,
            snap_distance: defaults.snap_distance,
            step_height: defaults.step_height,
            carry_platforms: defaults.carry_platforms,
        }
    }
}

impl TryFrom<AuthoredCharacter2d> for Character2dComponent {
    type Error = PhysicsError;
    fn try_from(value: AuthoredCharacter2d) -> Result<Self, Self::Error> {
        let settings = Self {
            skin: value.skin,
            max_iterations: value.max_iterations,
            up: value.up,
            max_slope_angle: value.max_slope_angle,
            snap_distance: value.snap_distance,
            step_height: value.step_height,
            carry_platforms: value.carry_platforms,
        };
        settings.options().validate()?;
        Ok(settings)
    }
}
