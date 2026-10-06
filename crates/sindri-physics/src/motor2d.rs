//! Sindri-owned motor policy, independent of backend joint types.
use serde::{Deserialize, Serialize};

/// Selects velocity drive or a damped position target for a 2D joint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotorMode2d {
    /// Drive at a target velocity within the force or torque cap.
    #[default]
    Velocity,
    /// Drive towards a target position using stiffness and damping.
    Position,
}

impl MotorMode2d {
    /// Supported modes, in authoring order.
    pub const ALL: [Self; 2] = [Self::Velocity, Self::Position];

    /// The spelling used by scene payloads and authoring choices.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Velocity => "velocity",
            Self::Position => "position",
        }
    }
}
