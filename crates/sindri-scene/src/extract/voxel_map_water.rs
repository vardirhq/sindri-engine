//! A map's water that moves and has depth (`map_water`).
//!
//! Each water square is drawn through the engine's own water shader
//! (`voxel_map_water.wgsl`). What it needs that a pixel cannot find out for
//! itself is worked out here once, with the squares: the water's colour at
//! that square's depth and distance from the shore, which neighbours are land
//! for the foam, and how much it sways, glints and foams.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use sindri_render::SpriteShader;

use crate::{VoxelGround, VoxelMapWater};

const SOURCE: &str = include_str!("voxel_map_water.wgsl");
/// How far from the shore is looked, in squares: further is open water.
const OPEN_WATER: i32 = 4;

/// The engine's water shader, checked the first time a map asks for it.
pub(super) fn shader() -> Option<SpriteShader> {
    static SHADER: OnceLock<Option<SpriteShader>> = OnceLock::new();
    SHADER
        .get_or_init(|| SpriteShader::new("builtin:map_water", SOURCE).ok())
        .clone()
}

/// Something for each of a chunk's squares, by column and row.
pub(super) type BySquare<T> = HashMap<(i32, i32), T>;

/// What a map's water settings come to for its squares.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Water {
    settings: VoxelMapWater,
    blocks: HashSet<String>,
}

/// One water square's look: its colour, and what its shader is handed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaterSquare {
    pub(super) colour: [f32; 3],
    pub(super) params: [f32; 4],
}

impl Water {
    /// The water a map's settings name, in its ground's blocks.
    pub(super) fn of(settings: &VoxelMapWater) -> Self {
        let blocks = settings.blocks.iter().cloned().collect();
        Self {
            settings: settings.clone(),
            blocks,
        }
    }

    /// Whether a column's top is water.
    fn is_water(&self, ground: &VoxelGround, column: i32, row: i32) -> bool {
        ground
            .surface(column, row)
            .is_some_and(|(level, _, _)| self.blocks.contains(&ground.block([column, level, row])))
    }

    /// The look of every water square from `columns` by `rows`, by square,
    /// and how much darker each square of land the water wets is drawn.
    pub(super) fn squares(
        &self,
        ground: &VoxelGround,
        columns: std::ops::Range<i32>,
        rows: std::ops::Range<i32>,
    ) -> (BySquare<WaterSquare>, BySquare<f32>) {
        let reach = OPEN_WATER + 1;
        let mut flooded = HashMap::new();
        for row in rows.start - reach..rows.end + reach {
            for column in columns.start - reach..columns.end + reach {
                flooded.insert((column, row), self.is_water(ground, column, row));
            }
        }
        let water = |column: i32, row: i32| flooded.get(&(column, row)).copied().unwrap_or(true);
        let amounts = self.amounts();
        let mut out = HashMap::new();
        let mut wet = HashMap::new();
        for row in rows {
            for column in columns.clone() {
                if !water(column, row) {
                    // The land at the water's edge is wet: darker beside it,
                    // a little darker at a corner of it.
                    let beside = [(0, -1), (1, 0), (0, 1), (-1, 0)]
                        .iter()
                        .any(|(dx, dy)| water(column + dx, row + dy));
                    let corner = [(1, -1), (1, 1), (-1, 1), (-1, -1)]
                        .iter()
                        .any(|(dx, dy)| water(column + dx, row + dy));
                    let share = if beside {
                        1.0
                    } else if corner {
                        0.5
                    } else {
                        0.0
                    };
                    if share > 0.0 && self.settings.wet > 0.0 {
                        wet.insert((column, row), 1.0 - self.settings.wet * share);
                    }
                    continue;
                }
                let shore = (0..OPEN_WATER)
                    .find(|ring| {
                        let ring = ring + 1;
                        (-ring..=ring).any(|d| {
                            !water(column + d, row - ring)
                                || !water(column + d, row + ring)
                                || !water(column - ring, row + d)
                                || !water(column + ring, row + d)
                        })
                    })
                    .unwrap_or(OPEN_WATER);
                let land = [
                    (0, -1, 1.0),
                    (1, 0, 2.0),
                    (0, 1, 4.0),
                    (-1, 0, 8.0),
                    (1, -1, 16.0),
                    (1, 1, 32.0),
                    (-1, 1, 64.0),
                    (-1, -1, 128.0),
                ]
                .iter()
                .filter(|(dx, dy, _)| !water(column + dx, row + dy))
                .map(|(_, _, bit)| bit)
                .sum::<f32>();
                let deep = self.deepness(ground, column, row, shore);
                let colour = std::array::from_fn(|channel| {
                    let [shallow, deep_colour] = [self.settings.shallow, self.settings.deep];
                    shallow[channel] + (deep_colour[channel] - shallow[channel]) * deep
                });
                #[allow(clippy::cast_precision_loss)]
                let params = [deep, land, shore as f32 * 16.0 + self.pace(), amounts];
                out.insert((column, row), WaterSquare { colour, params });
            }
        }
        (out, wet)
    }

    /// How deep a square looks, 0 at the shore to 1 at the settings' depth:
    /// its blocks of water, and shallower near the land whatever is under it.
    fn deepness(&self, ground: &VoxelGround, column: i32, row: i32, shore: i32) -> f32 {
        let Some((level, _, _)) = ground.surface(column, row) else {
            return 0.0;
        };
        let most = i32::try_from(self.settings.depth.max(1)).unwrap_or(1);
        let top = ground.voxel([column, level, row]);
        let blocks = (1..most)
            .take_while(|below| ground.voxel([column, level - below, row]) == top)
            .count();
        #[allow(clippy::cast_precision_loss)]
        let by_depth = (blocks + 1) as f32 / most as f32;
        #[allow(clippy::cast_precision_loss)]
        let by_shore = (shore as f32 + 0.5) / OPEN_WATER as f32;
        by_depth.min(by_shore.min(1.0)).clamp(0.0, 1.0)
    }

    /// How strongly what stands by it is mirrored in it.
    pub(super) fn reflections(&self) -> f32 {
        self.settings.reflections
    }

    /// How fast it moves, in quarters, 0 to 15.
    fn pace(&self) -> f32 {
        (self.settings.speed * 4.0).round().clamp(0.0, 15.0)
    }

    /// Sway, glints and foam, a fifteenth at a time, packed as the shader
    /// unpacks them.
    fn amounts(&self) -> f32 {
        let fifteenths = |amount: f32| (amount * 15.0).round().clamp(0.0, 15.0);
        fifteenths(self.settings.waves)
            + fifteenths(self.settings.glints) * 16.0
            + fifteenths(self.settings.foam) * 256.0
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_water_shader_compiles() {
        assert!(
            super::shader().is_some(),
            "{:?}",
            sindri_render::SpriteShader::new("water", super::SOURCE).err()
        );
    }
}
