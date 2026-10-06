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
