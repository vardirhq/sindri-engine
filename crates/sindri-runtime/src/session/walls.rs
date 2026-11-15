//! Walls on a map's unwalkable ground near play (`sindri.map_walls`).
//!
//! The land is walled sixteen squares by sixteen: every chunk within reach of
//! play holds a static 2D collider over each run of squares in a row that a
//! walker cannot stand on, and a chunk is taken down once it is a chunk past
//! that, so walking along the edge of the reach does not raise and lower the
//! same walls every step. They are ordinary entities, which is all 2D physics
//! needs to know of them.

use std::collections::{BTreeMap, HashMap};

use glam::Vec3;
use sindri_core::{EntityData, EntityId, Transform3D, World};
use sindri_scene::{MapWallsComponent, VoxelGround, VoxelWorldComponent};

use super::Session;
use super::scatter::{land_matrix, middle_of_play};

/// How many squares a side of one walled chunk holds.
const CHUNK: i32 = 16;

/// What one land's walls are built from, and the chunks standing.
#[derive(Clone)]
struct Walled {
    world: VoxelWorldComponent,
    walls: MapWallsComponent,
    ground: VoxelGround,
    middle: Option<[i32; 2]>,
    /// Each standing chunk's walls, by the chunk.
    chunks: HashMap<[i32; 2], Vec<EntityId>>,
}

/// Every walled land.
#[derive(Clone, Default)]
pub(super) struct Walls(BTreeMap<EntityId, Walled>);

impl Session {
    /// Raises the walls near play on every walled land and takes down those
    /// far from it.
    pub(super) fn step_walls(
        &mut self,
        world: &mut World,
        viewport: (f32, f32),
        problems: &mut Vec<String>,
    ) {
        let lands: Vec<(EntityId, MapWallsComponent, VoxelWorldComponent)> =
            match self.components.query::<MapWallsComponent>(world) {
                Ok(found) => found
                    .into_iter()
                    .filter(|(entity, _)| world.is_active(*entity))
                    .filter_map(|(entity, walls)| {
                        let land = self.components.get::<VoxelWorldComponent>(world, entity);
                        Some((entity, walls, land.ok()??))
                    })
                    .collect(),
                Err(error) => {
                    problems.push(format!("Map walls: {error}"));
                    return;
                }
            };
        let gone: Vec<EntityId> = self
            .walls
            .0
            .keys()
            .filter(|entity| !lands.iter().any(|(kept, _, _)| kept == *entity))
            .copied()
            .collect();
        for entity in gone {
            if let Some(walled) = self.walls.0.remove(&entity) {
                take_down(world, walled.chunks.into_values().flatten());
            }
        }
        for (entity, walls, land) in lands {
            if let Err(error) = walls.validate() {
                problems.push(format!("Map walls on {entity:?}: {error}"));
                continue;
            }
            let Some(middle) =
                middle_of_play(world, &self.components, entity, &walls.around, viewport)
            else {
                continue;
            };
            if let Err(problem) = self.wall(world, entity, walls, land, middle) {
                problems.push(format!("Map walls on {entity:?}: {problem}"));
            }
        }
    }

    fn wall(
        &mut self,
        world: &mut World,
        entity: EntityId,
        walls: MapWallsComponent,
        land: VoxelWorldComponent,
        middle: [i32; 2],
    ) -> Result<(), String> {
        let tile_sets = (!self.tile_sets.is_empty()).then_some(&self.tile_sets);
        let stale = self
            .walls
            .0
            .get(&entity)
            .is_none_or(|walled| walled.world != land || walled.walls != walls);
        if stale {
            // The ground or the walls changed: every chunk is raised again.
            let ground = VoxelGround::of(&land, tile_sets).map_err(|error| error.to_string())?;
            if let Some(old) = self.walls.0.remove(&entity) {
                take_down(world, old.chunks.into_values().flatten());
            }
            self.walls.0.insert(
                entity,
                Walled {
                    world: land,
                    walls,
                    ground,
                    middle: None,
                    chunks: HashMap::new(),
                },
            );
        }
        let walled = self.walls.0.get_mut(&entity).expect("inserted above");
        if walled.middle == Some(middle) {
            return Ok(());
        }
        walled.middle = Some(middle);
        let reach = i32::try_from(walled.walls.reach).unwrap_or(i32::MAX / 4);
        let low = [
            (middle[0] - reach).div_euclid(CHUNK),
            (middle[1] - reach).div_euclid(CHUNK),
        ];
        let high = [
            (middle[0] + reach).div_euclid(CHUNK),
            (middle[1] + reach).div_euclid(CHUNK),
        ];
        let far: Vec<[i32; 2]> = walled
            .chunks
            .keys()
            .filter(|chunk| {
                chunk[0] < low[0] - 1
                    || chunk[0] > high[0] + 1
                    || chunk[1] < low[1] - 1
                    || chunk[1] > high[1] + 1
            })
            .copied()
            .collect();
        for chunk in far {
            if let Some(standing) = walled.chunks.remove(&chunk) {
                take_down(world, standing);
            }
        }
        let root = land_matrix(world, entity);
        for chunk_row in low[1]..=high[1] {
            for chunk_column in low[0]..=high[0] {
                let chunk = [chunk_column, chunk_row];
                if walled.chunks.contains_key(&chunk) {
                    continue;
                }
                let raised = raise(world, walled, root, chunk);
                walled.chunks.insert(chunk, raised);
            }
        }
        Ok(())
    }
}

/// One chunk's walls: a static box over each unwalkable run of each row.
fn raise(world: &mut World, walled: &Walled, root: glam::Mat4, chunk: [i32; 2]) -> Vec<EntityId> {
    let (first, last) = (chunk[0] * CHUNK, chunk[0] * CHUNK + CHUNK - 1);
    let (scale, _, _) = root.to_scale_rotation_translation();
    let mut raised = Vec::new();
    for row in chunk[1] * CHUNK..chunk[1] * CHUNK + CHUNK {
        let runs = sindri_scene::unwalkable_runs(first, last, |column| {
            walled
                .ground
                .surface(column, row)
                .map(|(_, _, walkable)| walkable)
        });
        for [from, to] in runs {
            #[allow(clippy::cast_precision_loss)]
            let (middle, length) = ((from + to) as f32 / 2.0 + 0.5, (to - from + 1) as f32);
            #[allow(clippy::cast_precision_loss)]
            let at = root.transform_point3(Vec3::new(middle, -(row as f32) - 0.5, 0.0));
            let components = walled
                .walls
                .wall_components([length * scale.x / 2.0, scale.y / 2.0]);
            raised.push(world.spawn(EntityData {
                name: Some("Map wall".to_owned()),
                transform_3d: Some(Transform3D {
                    position: at.to_array(),
                    ..Transform3D::default()
                }),
                components: components.into_iter().collect(),
                ..EntityData::default()
            }));
        }
    }
    raised
}

fn take_down(world: &mut World, walls: impl IntoIterator<Item = EntityId>) {
    for wall in walls {
        // A wall something else already removed is simply gone.
        let _ = world.despawn_recursive(wall);
    }
}
