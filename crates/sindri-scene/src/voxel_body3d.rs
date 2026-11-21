//! A built voxel world as one rigid body: a ship, a station, a wreck.
//!
//! `sindri.physics3d.voxel_body` on a built `sindri.voxel_world` makes the
//! world's placed blocks the colliders of the entity's body, so the whole
//! structure moves, turns, collides and is pushed as one. The blocks are
//! merged into as few boxes as cover them exactly, in the world's own voxel
//! space, then scaled with the entity like any other collider; mass follows
//! the boxes. With a `sindri.physics3d.rigid_body` the body is whatever kind
//! that says, dynamic to fly; without one it is static. Placing or removing a
//! block rebuilds the colliders and keeps the body's motion.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sindri_core::SceneComponent;
use sindri_physics::{Collider3d, CollisionLayers};

use crate::{VoxelGeneratorDocument, VoxelWorldComponent};

/// The surface every block of a voxel body shares.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct VoxelBody3dComponent {
    #[serde(default = "default_friction")]
    pub friction: f32,
    #[serde(default)]
    pub restitution: f32,
    #[serde(default)]
    pub layers: CollisionLayers,
}

const fn default_friction() -> f32 {
    0.5
}

impl Default for VoxelBody3dComponent {
    fn default() -> Self {
        Self {
            friction: default_friction(),
            restitution: 0.0,
            layers: CollisionLayers::default(),
        }
    }
}

impl SceneComponent for VoxelBody3dComponent {
    const TYPE_NAME: &'static str = "sindri.physics3d.voxel_body";
}

/// The blocks a built world holds: its edits, later ones replacing earlier,
/// air removing.
fn solid_blocks(world: &VoxelWorldComponent) -> BTreeSet<[i32; 3]> {
    let mut cells = BTreeMap::new();
    for edit in &world.edits {
        cells.insert(edit.at, !edit.block.is_air());
    }
    cells
        .into_iter()
        .filter_map(|(at, solid)| solid.then_some(at))
        .collect()
}

/// The colliders a voxel body's blocks make, in the world's unscaled voxel
/// space: as few boxes as cover the blocks exactly, each block in exactly one.
///
/// # Errors
/// A world that is not built: a generated world has no edge to be a body.
pub fn voxel_body_pieces(
    world: &VoxelWorldComponent,
    body: &VoxelBody3dComponent,
) -> Result<Vec<Collider3d>, &'static str> {
    if !matches!(world.generator, VoxelGeneratorDocument::Built {}) {
        return Err("a voxel body is a built world: its generator must be `built`");
    }
    let usable = |value: f32| value.is_finite() && value >= 0.0;
    if !usable(body.friction) || !usable(body.restitution) {
        return Err("a voxel body's friction and restitution must be numbers of 0 or more");
    }
    let mut left = solid_blocks(world);
    let mut pieces = Vec::new();
    // Greedy merging in a fixed order: along X, then Z, then up, so the same
    // blocks always make the same boxes.
    let mut order: Vec<[i32; 3]> = left.iter().copied().collect();
    order.sort_by_key(|&[x, y, z]| (y, z, x));
    for start in order {
        if !left.contains(&start) {
            continue;
        }
        let [x0, y0, z0] = start;
        let mut width = 1;
        while left.contains(&[x0 + width, y0, z0]) {
            width += 1;
        }
        let row = |y: i32, z: i32, left: &BTreeSet<[i32; 3]>| {
            (x0..x0 + width).all(|x| left.contains(&[x, y, z]))
        };
        let mut depth = 1;
        while row(y0, z0 + depth, &left) {
            depth += 1;
        }
        let mut height = 1;
        while (z0..z0 + depth).all(|z| row(y0 + height, z, &left)) {
            height += 1;
        }
        for y in y0..y0 + height {
            for z in z0..z0 + depth {
                for x in x0..x0 + width {
                    left.remove(&[x, y, z]);
                }
            }
        }
        #[allow(clippy::cast_precision_loss)] // Block counts are small.
        let size = [width as f32, height as f32, depth as f32];
        #[allow(clippy::cast_precision_loss)]
        let corner = [x0 as f32, y0 as f32, z0 as f32];
        pieces.push(Collider3d {
            offset: [0, 1, 2].map(|axis| corner[axis] + size[axis] / 2.0),
            layers: body.layers,
            friction: body.friction,
            restitution: body.restitution,
            ..Collider3d::cuboid(size.map(|edge| edge / 2.0))
        });
    }
    Ok(pieces)
}

#[cfg(test)]
mod tests;
