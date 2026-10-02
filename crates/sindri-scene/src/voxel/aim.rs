//! What the pointer is on, and where a block clicked there would go.
//!
//! The host answers this before scripts run, for the same reason it answers
//! where the screen elements are: which block a person is pointing at is a
//! fact about this frame, decided by a camera and a pointer that a script does
//! not own. A script that had to work it out would need the projection matrix,
//! and handing that to gameplay makes every game responsible for inverting its
//! own camera.
//!
//! The editor reaches the same picking directly, because it *is* a tool and
//! already holds a viewport. This is the same call with the search for a grid
//! in front of it.

use glam::Mat4;
use sindri_core::{ComponentSchemaRegistry, EntityId, TileFace, World};
use sindri_grid::GridCoord3;

use crate::components::{TileGridComponent, TileVolumeComponent};
use crate::{TileSetBindings, VoxelGround, voxel_space};

use super::{VoxelHit, pick, ray_at_viewport};

/// Which block the pointer is over, on which grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VolumeAim {
    /// The entity carrying the grid and volume that was hit.
    pub grid: EntityId,
    /// The block under the pointer: what a right-click would remove.
    pub cell: GridCoord3,
    /// The side of it being looked at.
    pub face: TileFace,
    /// The empty cell against that side: what a left-click would fill.
    pub against: GridCoord3,
}

/// How far a click reaches, in cells.
///
/// Far enough to cross any island a person can see at once, and short enough
/// that a ray leaving the volume stops rather than walking the grid forever.
pub const REACH: f32 = 512.0;

/// The block under a point on the picture, if there is one.
///
/// `viewport_point` is the fraction across and down the picture. `None` covers
/// every way there is nothing to answer -- no solid grid in the world, the
/// pointer outside the picture, a camera that cannot be inverted, a ray that
/// meets no block -- because to a caller they are one situation: the person is
/// not pointing at a block.
#[must_use]
pub fn aim_at(
    world: &World,
    components: &ComponentSchemaRegistry,
    view_projection: Mat4,
    viewport_point: [f32; 2],
) -> Option<VolumeAim> {
    aim_at_with(world, components, None, view_projection, viewport_point)
}

/// The same, with the tile sets that say what a voxel world's blocks are.
///
/// A voxel world built from a block set can only be aimed at with them: which
/// voxels are blocks and which are air is a question the set answers.
#[must_use]
pub fn aim_at_with(
    world: &World,
    components: &ComponentSchemaRegistry,
    tile_sets: Option<&TileSetBindings>,
    view_projection: Mat4,
    viewport_point: [f32; 2],
) -> Option<VolumeAim> {
    let grids = components.query::<TileGridComponent>(world).ok()?;
    for (entity, grid) in grids {
        // Only a grid of boxes has sides to click on. A projected one draws a
        // picture of blocks, and the picture has no near side.
        let Some(cell_size) = grid.solid_cell() else {
            continue;
        };
        let transform = world.world_transform(entity).unwrap_or_default();
        let Some((origin, direction)) = ray_at_viewport(transform, view_projection, viewport_point)
        else {
            continue;
        };
        // A grid whose ground is a voxel world is aimed at through the world,
        // whose cells are the grid's: a column, a row and a level are a
        // voxel's X, Z and height.
        if let Ok(Some(ground)) = VoxelGround::of_entity(world, components, entity, tile_sets) {
            let into_voxels = voxel_space(world, components, entity).inverse();
            let hit = ground.raycast(
                into_voxels.transform_point3(origin),
                into_voxels.transform_vector3(direction),
                REACH,
            );
            if let Some(hit) = hit {
                let cell = |at: [i32; 3]| GridCoord3::new(at[0], at[2], at[1]);
                return Some(VolumeAim {
                    grid: entity,
                    cell: cell(hit.cell),
                    face: face_of(hit.normal),
                    against: cell(hit.before()),
                });
            }
            continue;
        }
        let Some(volume) = components
            .get::<TileVolumeComponent>(world, entity)
            .ok()
            .flatten()
        else {
            continue;
        };
        if let Some(VoxelHit { cell, face }) = pick(&volume, cell_size, origin, direction, REACH) {
            let hit = VoxelHit { cell, face };
            return Some(VolumeAim {
                grid: entity,
                cell,
                face,
                against: hit.against(),
            });
        }
    }
    None
}

/// The side of a block a ray came in through, from which way out of the
/// block it came: a voxel's up is a grid's top, its +Z a grid's south.
const fn face_of(normal: [i32; 3]) -> TileFace {
    match normal {
        [1, _, _] => TileFace::East,
        [-1, _, _] => TileFace::West,
        [_, -1, _] => TileFace::Bottom,
        [_, _, 1] => TileFace::South,
        [_, _, -1] => TileFace::North,
        _ => TileFace::Top,
    }
}
