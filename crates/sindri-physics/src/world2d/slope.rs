//! Shared support classification and slope-limited tangent projection.

use super::r2;

/// Constructed only after validating and normalizing the caller's up direction.
#[derive(Clone, Copy)]
pub(super) struct SlopeLimit2d {
    up: r2::Vector,
    minimum_up_dot: f32,
}

impl SlopeLimit2d {
    pub(super) fn new(up: r2::Vector, max_angle: f32) -> Self {
        Self {
            up,
            minimum_up_dot: max_angle.cos().max(0.0),
        }
    }

    pub(super) fn walkable(self, normal: r2::Vector) -> bool {
        // Narrow-phase normals approximate curved/rotated geometry. Keep the
        // same tolerance for motion limits and support classification.
        normal.dot(self.up) + 0.0001 >= self.minimum_up_dot
    }

    pub(super) fn project(self, wanted: r2::Vector, normal: r2::Vector) -> r2::Vector {
        let tangent = wanted - normal * wanted.dot(normal).min(0.0);
        let rise = tangent.dot(self.up);
        let allowed_rise = wanted.dot(self.up).max(0.0);
        if normal.dot(self.up) > f32::EPSILON && !self.walkable(normal) && rise > allowed_rise {
            // Scaling the tangent preserves separation. Merely removing its
            // upward component would point back into the steep surface.
            tangent * (allowed_rise / rise)
        } else {
            tangent
        }
    }
}
