//! Exact, bounded section-local geometry without physics or renderer types.

use crate::{LocalVoxelCoord, SECTION_VOLUME, VOXEL_STEPS, VoxelId, VoxelSection, VoxelShape};

const EDGE: u8 = 16;

/// One occupied box, in section-local sixteenths of a voxel (0..=256).
/// Integer bounds retain exact slabs/posts even at distant world coordinates.
/// Hosts apply the section origin and world transform when building colliders.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SectionCollisionBox {
    pub min: [u16; 3],
    pub max: [u16; 3],
}

/// A collision policy supplied a box outside its voxel or with zero thickness.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VoxelCollisionError {
    pub voxel: VoxelId,
    pub shape: VoxelShape,
}

impl std::fmt::Display for VoxelCollisionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "invalid collision shape {:?} for voxel {}",
            self.shape,
            self.voxel.value()
        )
    }
}

impl std::error::Error for VoxelCollisionError {}

/// Compiles one resident section into disjoint boxes covering exactly its
/// collidable occupancy. Air never calls the policy; `None` makes another block
/// noncolliding. The policy owns solidity independently of render transparency.
///
/// Full cubes merge greedily in X, then Z, then Y, including across material IDs.
/// Partial shapes retain their exact individual bounds. Output is deterministic
/// for a stable policy, has at most [`SECTION_VOLUME`] boxes and never samples a
/// neighbour or generator. It does not guarantee a minimum box count.
/// This is geometry only: hosts needing per-block physics coefficients or
/// layers must compile separate policies/groups before merging those settings.
///
/// Hosts retain the result by occupancy/policy revision, rebuild entering or
/// edited resident sections, release departing sections and bound frame work.
/// This function neither owns residency nor inserts solver bodies.
///
/// # Errors
/// Returns [`VoxelCollisionError`] for an inverted, empty or out-of-voxel shape.
pub fn compile_section_collision(
    section: &VoxelSection,
    shape_of: impl Fn(VoxelId) -> Option<VoxelShape>,
) -> Result<Vec<SectionCollisionBox>, VoxelCollisionError> {
    let mut full = [false; SECTION_VOLUME];
    let mut boxes = Vec::new();
    for y in 0..EDGE {
        for z in 0..EDGE {
            for x in 0..EDGE {
                let cell = LocalVoxelCoord::new(x, y, z);
                let voxel = section.get(cell);
                if voxel.is_air() {
                    continue;
                }
                let Some(shape) = shape_of(voxel) else {
                    continue;
                };
                if (0..3)
                    .any(|axis| shape.min[axis] >= shape.max[axis] || shape.max[axis] > VOXEL_STEPS)
                {
                    return Err(VoxelCollisionError { voxel, shape });
                }
                if shape.is_full() {
                    full[cell.index()] = true;
                } else {
                    let origin = [x, y, z].map(|axis| u16::from(axis) * u16::from(VOXEL_STEPS));
                    boxes.push(SectionCollisionBox {
                        min: std::array::from_fn(|axis| origin[axis] + u16::from(shape.min[axis])),
                        max: std::array::from_fn(|axis| origin[axis] + u16::from(shape.max[axis])),
                    });
                }
            }
        }
    }
    merge_full(&mut full, &mut boxes);
    Ok(boxes)
}

fn merge_full(full: &mut [bool; SECTION_VOLUME], boxes: &mut Vec<SectionCollisionBox>) {
    for y in 0..EDGE {
        for z in 0..EDGE {
            for x in 0..EDGE {
                let min = [x, y, z];
                if !full[LocalVoxelCoord::new(x, y, z).index()] {
                    continue;
                }
                let mut max = [x + 1, y + 1, z + 1];
                for axis in [0, 2, 1] {
                    while max[axis] < EDGE {
                        let mut slab_min = min;
                        slab_min[axis] = max[axis];
                        let mut slab_max = max;
                        slab_max[axis] += 1;
                        if !cells(slab_min, slab_max).all(|cell| full[cell.index()]) {
                            break;
                        }
                        max[axis] += 1;
                    }
                }
                for cell in cells(min, max) {
                    full[cell.index()] = false;
                }
                boxes.push(SectionCollisionBox {
                    min: min.map(|axis| u16::from(axis) * u16::from(VOXEL_STEPS)),
                    max: max.map(|axis| u16::from(axis) * u16::from(VOXEL_STEPS)),
                });
            }
        }
    }
}

fn cells(min: [u8; 3], max: [u8; 3]) -> impl Iterator<Item = LocalVoxelCoord> {
    (min[1]..max[1]).flat_map(move |y| {
        (min[2]..max[2])
            .flat_map(move |z| (min[0]..max[0]).map(move |x| LocalVoxelCoord::new(x, y, z)))
    })
}

#[cfg(test)]
#[path = "collision_tests.rs"]
mod tests;
