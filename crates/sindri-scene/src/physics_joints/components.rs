//! Stable scene endpoints and kind-specific settings.
use serde::{Deserialize, Serialize};
use sindri_core::SceneComponent;
use sindri_physics::{HingeSettings2d, SliderSettings2d, SpringSettings2d};

/// A separate entity owns the constraint, so several joints can connect a body.
/// Endpoints are stable scene IDs, never serialized runtime handles. Empty,
/// missing or inactive endpoints suspend the joint until they become available.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct DistanceJoint2dComponent {
    #[serde(default)]
    pub first: String,
    #[serde(default)]
    pub second: String,
    /// Suspends this constraint while retaining its endpoints and settings.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_distance")]
    pub max_distance: f32,
}

const fn default_enabled() -> bool {
    true
}

const fn default_distance() -> f32 {
    1.0
}

impl SceneComponent for DistanceJoint2dComponent {
    const TYPE_NAME: &'static str = "sindri.physics2d.distance_joint";
}

/// Body-local anchors, optional relative angle limits and a velocity motor.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct HingeJoint2dComponent {
    #[serde(default)]
    pub first: String,
    #[serde(default)]
    pub second: String,
    /// Suspends this constraint while retaining its endpoints and settings.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(flatten)]
    pub settings: HingeSettings2d,
}

impl SceneComponent for HingeJoint2dComponent {
    const TYPE_NAME: &'static str = "sindri.physics2d.hinge_joint";
}

/// Aligned local axes, signed travel limits and an optional linear drive.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SliderJoint2dComponent {
    #[serde(default)]
    pub first: String,
    #[serde(default)]
    pub second: String,
    /// Suspends this constraint while retaining its endpoints and settings.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(flatten)]
    pub settings: SliderSettings2d,
}
impl SceneComponent for SliderJoint2dComponent {
    const TYPE_NAME: &'static str = "sindri.physics2d.slider_joint";
}

/// A force-based spring between local anchors, with independent damping.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SpringJoint2dComponent {
    #[serde(default)]
    pub first: String,
    #[serde(default)]
    pub second: String,
    /// Suspends this constraint while retaining its endpoints and settings.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(flatten)]
    pub settings: SpringSettings2d,
}
impl SceneComponent for SpringJoint2dComponent {
    const TYPE_NAME: &'static str = "sindri.physics2d.spring_joint";
}

impl Default for DistanceJoint2dComponent {
    fn default() -> Self {
        Self {
            first: String::new(),
            second: String::new(),
            enabled: true,
            max_distance: default_distance(),
        }
    }
}
impl Default for HingeJoint2dComponent {
    fn default() -> Self {
        Self {
            first: String::new(),
            second: String::new(),
            enabled: true,
            settings: HingeSettings2d::default(),
        }
    }
}
impl Default for SliderJoint2dComponent {
    fn default() -> Self {
        Self {
            first: String::new(),
            second: String::new(),
            enabled: true,
            settings: SliderSettings2d::default(),
        }
    }
}
impl Default for SpringJoint2dComponent {
    fn default() -> Self {
        Self {
            first: String::new(),
            second: String::new(),
            enabled: true,
            settings: SpringSettings2d::default(),
        }
    }
}

impl DistanceJoint2dComponent {
    /// Validates the maximum distance independently of endpoint availability.
    /// # Errors
    /// A non-finite or non-positive maximum distance.
    pub fn validate(&self) -> Result<(), sindri_physics::PhysicsError> {
        if !self.max_distance.is_finite() {
            return Err(sindri_physics::PhysicsError::NonFinite("max_distance"));
        }
        if self.max_distance <= 0.0 {
            return Err(sindri_physics::PhysicsError::NonPositive("max_distance"));
        }
        Ok(())
    }
}
