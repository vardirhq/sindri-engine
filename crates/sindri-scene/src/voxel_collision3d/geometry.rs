use sindri_physics::Collider3d;
use sindri_voxel::{SECTION_EDGE, SectionCollisionBox, SectionCoord, VOXEL_STEPS};

use super::{VoxelCollisionError3d, VoxelCollisionSettings3d};

// Integer section coordinates and finite section-local bounds are converted to
// the physics engine's f32 coordinate domain; backend validation checks overflow.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
pub(super) fn pieces(
    coord: SectionCoord,
    boxes: &[SectionCollisionBox],
    scale: [f32; 3],
    settings: VoxelCollisionSettings3d,
) -> Result<Vec<Collider3d>, VoxelCollisionError3d> {
    if !scale.into_iter().all(|axis| axis.is_finite() && axis > 0.0) {
        return Err(VoxelCollisionError3d::Scale);
    }
    let origin = [coord.x, coord.y, coord.z].map(|axis| f64::from(axis) * f64::from(SECTION_EDGE));
    Ok(boxes
        .iter()
        .map(|bounds| {
            let min = bounds
                .min
                .map(|axis| f64::from(axis) / f64::from(VOXEL_STEPS));
            let max = bounds
                .max
                .map(|axis| f64::from(axis) / f64::from(VOXEL_STEPS));
            let half = std::array::from_fn(|axis| {
                ((max[axis] - min[axis]) * 0.5 * f64::from(scale[axis])) as f32
            });
            let offset = std::array::from_fn(|axis| {
                ((origin[axis] + (min[axis] + max[axis]) * 0.5) * f64::from(scale[axis])) as f32
            });
            Collider3d {
                offset,
                layers: settings.layers,
                friction: settings.friction,
                restitution: settings.restitution,
                sensor: settings.sensor,
                ..Collider3d::cuboid(half)
            }
        })
        .collect())
}
