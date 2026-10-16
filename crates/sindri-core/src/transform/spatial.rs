//! Rotation-only directions and checked aim/orbit operations.

use super::Transform3D;
use super::quaternion::{multiply, normalized, rotate};

/// Invalid input to a spatial transform operation. Failed operations leave it unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TransformError {
    #[error("transform operation requires finite values")]
    NonFinite,
    #[error("look-at target must differ from the transform position")]
    CoincidentTarget,
    #[error("rotation axis must be nonzero")]
    ZeroAxis,
}

impl Transform3D {
    /// Unit -Z facing direction in the coordinate space of this transform.
    /// Scale is ignored. Invalid or zero quaternions use the identity rotation.
    #[must_use]
    pub fn forward(self) -> [f32; 3] {
        rotate(self.rotation, [0.0, 0.0, -1.0])
    }

    /// Unit +X direction, ignoring scale.
    #[must_use]
    pub fn right(self) -> [f32; 3] {
        rotate(self.rotation, [1.0, 0.0, 0.0])
    }

    /// Unit +Y direction, ignoring scale.
    #[must_use]
    pub fn up(self) -> [f32; 3] {
        rotate(self.rotation, [0.0, 1.0, 0.0])
    }

    /// Faces a target in this transform's coordinate space with Y up and zero roll.
    /// At the poles yaw is zero. Position and scale are unchanged.
    ///
    /// # Errors
    /// Rejects non-finite inputs and a target at the current position.
    pub fn look_at(&mut self, target: [f32; 3]) -> Result<(), TransformError> {
        if !target.into_iter().chain(self.position).all(f32::is_finite) {
            return Err(TransformError::NonFinite);
        }
        let delta =
            std::array::from_fn::<_, 3, _>(|i| f64::from(target[i]) - f64::from(self.position[i]));
        let horizontal = delta[0].hypot(delta[2]);
        if horizontal <= 0.0 && delta[1].abs() <= 0.0 {
            return Err(TransformError::CoincidentTarget);
        }
        let yaw = if horizontal <= 0.0 {
            0.0
        } else {
            (-delta[0]).atan2(-delta[2])
        };
        self.set_yaw_pitch_roll_radians([narrow(yaw), narrow(delta[1].atan2(horizontal)), 0.0]);
        Ok(())
    }

    /// Orbits a pivot about an axis in this transform's coordinate space, in radians.
    /// Both position and orientation rotate; scale and the layer declaration remain.
    ///
    /// # Errors
    /// Rejects non-finite input/output and a zero axis.
    /// The layer check belongs to the caller; this operation computes the candidate.
    pub fn rotate_around(
        &mut self,
        pivot: [f32; 3],
        axis: [f32; 3],
        radians: f32,
    ) -> Result<(), TransformError> {
        if !pivot
            .into_iter()
            .chain(axis)
            .chain(self.position)
            .chain(self.rotation)
            .chain([radians])
            .all(f32::is_finite)
        {
            return Err(TransformError::NonFinite);
        }
        let length = axis
            .into_iter()
            .map(|v| f64::from(v).powi(2))
            .sum::<f64>()
            .sqrt();
        if length <= 0.0 {
            return Err(TransformError::ZeroAxis);
        }
        if radians.abs() <= 0.0 {
            return Ok(());
        }
        let (sin, cos) = (radians * 0.5).sin_cos();
        let unit = axis.map(|v| narrow(f64::from(v) / length));
        let turn = [unit[0] * sin, unit[1] * sin, unit[2] * sin, cos];
        // Compute the offset in double precision to avoid overflowing two finite positions.
        let offset =
            std::array::from_fn::<_, 3, _>(|i| f64::from(self.position[i]) - f64::from(pivot[i]));
        let [axis_x, axis_y, axis_z] = unit.map(f64::from);
        let [vx, vy, vz] = offset;
        let (sine, cosine) = f64::from(radians).sin_cos();
        let dot = axis_x * vx + axis_y * vy + axis_z * vz;
        let turned = [
            vx * cosine + (axis_y * vz - axis_z * vy) * sine + axis_x * dot * (1.0 - cosine),
            vy * cosine + (axis_z * vx - axis_x * vz) * sine + axis_y * dot * (1.0 - cosine),
            vz * cosine + (axis_x * vy - axis_y * vx) * sine + axis_z * dot * (1.0 - cosine),
        ];
        let position = std::array::from_fn(|i| narrow(f64::from(pivot[i]) + turned[i]));
        if !position.into_iter().all(f32::is_finite) {
            return Err(TransformError::NonFinite);
        }
        self.position = position;
        self.rotation = normalized(multiply(turn, normalized(self.rotation)));
        Ok(())
    }
}

// Angles and normalized components fit f32; position overflow is checked before mutation.
#[allow(clippy::cast_possible_truncation)]
fn narrow(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
#[path = "spatial_tests.rs"]
mod tests;
