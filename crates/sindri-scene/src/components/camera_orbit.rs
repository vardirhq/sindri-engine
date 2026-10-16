//! Authored third-person camera settings, independent of the 2D follow path.

use serde::{Deserialize, Serialize};
use sindri_core::{SceneComponent, SceneEntityId};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CameraOrbitComponent {
    pub target: SceneEntityId,
    #[serde(default)]
    pub offset: [f32; 3],
    /// Radians about world Y. Zero places the camera on the target's +Z side.
    #[serde(default)]
    pub yaw: f32,
    /// Radians above the target's horizontal plane, strictly between the poles.
    #[serde(default = "pitch")]
    pub pitch: f32,
    #[serde(default = "distance")]
    pub distance: f32,
    /// Exponential position smoothing per second; zero snaps to the desired pose.
    #[serde(default = "smoothing")]
    pub smoothing: f32,
    /// Collider membership bits to consider. Zero disables obstruction queries.
    #[serde(default = "mask")]
    pub collision_mask: u32,
    /// World units to stay in front of an obstruction along the sight line.
    #[serde(default = "padding")]
    pub collision_padding: f32,
}

const fn pitch() -> f32 {
    0.35
}
const fn distance() -> f32 {
    6.0
}
const fn smoothing() -> f32 {
    8.0
}
const fn mask() -> u32 {
    u32::MAX
}
const fn padding() -> f32 {
    0.2
}

impl CameraOrbitComponent {
    #[must_use]
    pub fn new(target: SceneEntityId) -> Self {
        Self {
            target,
            offset: [0.0; 3],
            yaw: 0.0,
            pitch: pitch(),
            distance: distance(),
            smoothing: smoothing(),
            collision_mask: mask(),
            collision_padding: padding(),
        }
    }

    /// Whether these settings describe a finite, non-degenerate orbit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.offset
            .into_iter()
            .chain([
                self.yaw,
                self.pitch,
                self.distance,
                self.smoothing,
                self.collision_padding,
            ])
            .all(f32::is_finite)
            && self.pitch.abs() < std::f32::consts::FRAC_PI_2
            && self.distance > 0.0
            && self.smoothing >= 0.0
            && self.collision_padding >= 0.0
            && self.collision_padding < self.distance
    }
}

impl SceneComponent for CameraOrbitComponent {
    const TYPE_NAME: &'static str = "sindri.camera.orbit";
}
