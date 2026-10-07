//! Walking a ray through a voxel world, voxel by voxel.

use super::{VoxelGround, VoxelWorldHit};

impl VoxelGround {
    /// The first solid voxel a ray meets, and the face it came in through.
    ///
    /// `origin` and `direction` are in the world's own voxel space, where a
    /// voxel spans one unit from its coordinate. Walks voxel by voxel, so the
    /// answer is exact at edges and corners, and gives up after `reach` units.
    #[must_use]
    pub fn raycast(
        &self,
        origin: glam::Vec3,
        direction: glam::Vec3,
        reach: f32,
    ) -> Option<VoxelWorldHit> {
        let length = direction.length();
        if !length.is_finite() || length <= f32::EPSILON || !origin.is_finite() {
            return None;
        }
        let direction = direction / length;
        let cell = origin.floor();
        #[allow(clippy::cast_possible_truncation)]
        let mut at = [cell.x as i32, cell.y as i32, cell.z as i32];
        let step = [
            step_of(direction.x),
            step_of(direction.y),
            step_of(direction.z),
        ];
        let delta = [
            crossing(direction.x),
            crossing(direction.y),
            crossing(direction.z),
        ];
        let mut next = [
            first_crossing(origin.x, direction.x),
            first_crossing(origin.y, direction.y),
            first_crossing(origin.z, direction.z),
        ];
        let mut normal = [0; 3];
        let mut travelled = 0.0;
        while travelled <= reach {
            if self.footing(self.voxel(at)).is_some() {
                return Some(VoxelWorldHit { cell: at, normal });
            }
            let axis = if next[0] <= next[1] && next[0] <= next[2] {
                0
            } else if next[1] <= next[2] {
                1
            } else {
                2
            };
            travelled = next[axis];
            next[axis] += delta[axis];
            at[axis] += step[axis];
            normal = [0; 3];
            normal[axis] = -step[axis];
        }
        None
    }
}

const fn step_of(direction: f32) -> i32 {
    if direction > 0.0 {
        1
    } else if direction < 0.0 {
        -1
    } else {
        0
    }
}

/// How far along the ray one whole voxel is on an axis.
fn crossing(direction: f32) -> f32 {
    if direction == 0.0 {
        f32::INFINITY
    } else {
        (1.0 / direction).abs()
    }
}

/// How far along the ray the first voxel boundary on an axis is.
fn first_crossing(origin: f32, direction: f32) -> f32 {
    if direction > 0.0 {
        (origin.floor() + 1.0 - origin) / direction
    } else if direction < 0.0 {
        (origin - origin.floor()) / -direction
    } else {
        f32::INFINITY
    }
}
