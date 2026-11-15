//! What stands on a generated land, scattered by seed (`sindri.scatter`).
//!
//! A voxel world carries this to be dressed: trees in the deadwood, boulders in
//! the barrens, reeds in the bog. Each rule names the ground it grows on and
//! the prefabs it makes, and how thickly. Where each one stands is the seed's
//! answer, so the same land is dressed the same way every time it is seen, and
//! nothing about it is stored but what was taken away.
//!
//! The engine makes the things near the middle of play and unmakes them once it
//! has gone on (`sindri-runtime`'s scatter), so a land as wide as the generator
//! is costs what is near. They are ordinary entities from ordinary prefabs: a
//! tree stops a walker and its shadow falls like any other's. One a script
//! removes, a tree felled for its wood, is remembered in `taken` and does not
//! grow back.

use serde::{Deserialize, Serialize};
use sindri_core::SceneComponent;

use crate::VoxelGround;

/// The land a voxel world's scatter dresses.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ScatterComponent {
    /// The entity whose place is the middle of what is dressed, by its scene
    /// ID: the one the player walks. Empty, the world camera's.
    pub around: String,
    /// How many squares out from the middle things stand. Further, they are
    /// unmade, and made again on the way back.
    pub reach: u32,
    /// Which arrangement this land wears.
    pub seed: u64,
    pub rules: Vec<ScatterRule>,
    /// Round places nothing grows in: a column, a row and a radius, in
    /// squares. Where play starts, round a lamp placed by hand.
    pub clearings: Vec<[f32; 3]>,
    /// What has been taken away and does not come back: a square's column and
    /// row, and the rule that put it there.
    pub taken: Vec<[i32; 3]>,
}

impl Default for ScatterComponent {
    /// One rule to fill in: what a tool offers when the component is added,
    /// and the shape it describes a rule's fields by.
    fn default() -> Self {
        Self {
            around: String::new(),
            reach: 24,
            seed: 0,
            rules: vec![ScatterRule {
                on: vec![String::new()],
                prefabs: vec![String::new()],
                ..ScatterRule::default()
            }],
            clearings: Vec::new(),
            taken: Vec::new(),
        }
    }
}

impl SceneComponent for ScatterComponent {
    const TYPE_NAME: &'static str = "sindri.scatter";
}

/// One kind of thing, and where it grows.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ScatterRule {
    /// The blocks it stands on, by name: a forest's ground, not the road's.
    pub on: Vec<String>,
    /// The prefabs it makes, one chosen by the seed for each place.
    pub prefabs: Vec<String>,
    /// How far apart, in squares: the land is cut into squares of this side,
    /// and each may hold one.
    pub spacing: u32,
    /// How likely each of those is to hold one, 0 to 1.
    pub chance: f32,
    /// Whether one may be mirrored, so five trees are ten.
    pub flip: bool,
}

impl Default for ScatterRule {
    fn default() -> Self {
        Self {
            on: Vec::new(),
            prefabs: Vec::new(),
            spacing: 3,
            chance: 0.5,
            flip: false,
        }
    }
}

/// The most a rule may space its things: past this a rule is a landmark, and a
/// landmark is placed by hand.
pub const MOST_SPACING: u32 = 64;
/// The furthest out a scatter reaches.
pub const MOST_REACH: u32 = 128;

/// Why a scatter was refused.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ScatterError {
    #[error("`reach` must be from 1 to {MOST_REACH}")]
    Reach,
    #[error("rule {0}'s `spacing` must be from 1 to {MOST_SPACING}")]
    Spacing(usize),
    #[error("rule {0}'s `chance` must be from 0 to 1")]
    Chance(usize),
}

impl ScatterComponent {
    /// The scatter, or why it would place nothing sensible. A rule that names
    /// no prefab yet is not refused: it makes nothing until it does.
    ///
    /// # Errors
    /// A reach or a spacing out of range, or a chance outside 0 to 1.
    pub fn validate(&self) -> Result<(), ScatterError> {
        if self.reach == 0 || self.reach > MOST_REACH {
            return Err(ScatterError::Reach);
        }
        for (index, rule) in self.rules.iter().enumerate() {
            if rule.spacing == 0 || rule.spacing > MOST_SPACING {
                return Err(ScatterError::Spacing(index));
            }
            if !(0.0..=1.0).contains(&rule.chance) {
                return Err(ScatterError::Chance(index));
            }
        }
        Ok(())
    }
}

/// One thing the scatter stands somewhere.
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    /// The square, as column and row, and the rule: what `taken` remembers.
    pub key: [i32; 3],
    /// The prefab it is made from.
    pub prefab: String,
    /// Where in the square it stands, from the square's middle, up the map
    /// north, within a third of a square either way.
    pub offset: [f32; 2],
    /// Whether it is mirrored.
    pub flipped: bool,
}

/// Everything the scatter stands on the squares from `min` to `max`
/// (inclusive, column and row), that has not been taken.
///
/// Where two rules choose one square, the first rule's thing stands there.
#[must_use]
pub fn placements(
    scatter: &ScatterComponent,
    ground: &VoxelGround,
    min: [i32; 2],
    max: [i32; 2],
) -> Vec<Placement> {
    let mut out: Vec<Placement> = Vec::new();
    let mut used = std::collections::HashSet::new();
    for (index, rule) in scatter.rules.iter().enumerate() {
        let Ok(rule_index) = i32::try_from(index) else {
            break;
        };
        let Ok(spacing) = i32::try_from(rule.spacing.max(1)) else {
            continue;
        };
        let prefabs: Vec<&String> = rule
            .prefabs
            .iter()
            .filter(|p| !p.trim().is_empty())
            .collect();
        if prefabs.is_empty() {
            continue;
        }
        for patch_row in min[1].div_euclid(spacing)..=max[1].div_euclid(spacing) {
            for patch_column in min[0].div_euclid(spacing)..=max[0].div_euclid(spacing) {
                let mut roll = Roll::new(scatter.seed, patch_column, patch_row, rule_index);
                if roll.unit() >= rule.chance {
                    continue;
                }
                let column = patch_column * spacing + roll.below(spacing);
                let row = patch_row * spacing + roll.below(spacing);
                if column < min[0] || column > max[0] || row < min[1] || row > max[1] {
                    continue;
                }
                let key = [column, row, rule_index];
                if scatter.taken.contains(&key) || in_a_clearing(scatter, column, row) {
                    continue;
                }
                if !used.insert((column, row)) {
                    continue;
                }
                let Some((level, _, walkable)) = ground.surface(column, row) else {
                    continue;
                };
                if !walkable || !rule.on.contains(&ground.block([column, level, row])) {
                    continue;
                }
                let pick = roll.below(i32::try_from(prefabs.len()).unwrap_or(1));
                let prefab = prefabs[usize::try_from(pick).unwrap_or(0)].clone();
                let offset = [(roll.unit() - 0.5) * 0.66, (roll.unit() - 0.5) * 0.66];
                let flipped = rule.flip && roll.unit() < 0.5;
                out.push(Placement {
                    key,
                    prefab,
                    offset,
                    flipped,
                });
            }
        }
    }
    out
}

/// Whether a square's middle is inside one of the scatter's clearings.
fn in_a_clearing(scatter: &ScatterComponent, column: i32, row: i32) -> bool {
    #[allow(clippy::cast_precision_loss)]
    let (x, y) = (column as f32 + 0.5, row as f32 + 0.5);
    scatter
        .clearings
        .iter()
        .any(|[cx, cy, radius]| (x - cx).hypot(y - cy) < *radius)
}

/// A run of numbers from a seed and a place: the same place, the same run.
struct Roll(u64);

impl Roll {
    fn new(seed: u64, column: i32, row: i32, rule: i32) -> Self {
        Self(
            seed ^ u64::from(column.cast_unsigned()).rotate_left(17)
                ^ u64::from(row.cast_unsigned()).rotate_left(41)
                ^ u64::from(rule.cast_unsigned()).wrapping_mul(0xD6E8_FEB8_6659_FD93),
        )
    }

    fn next(&mut self) -> u64 {
        // SplitMix64.
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number from 0 up to 1.
    fn unit(&mut self) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        let unit = (self.next() >> 40) as f32 / (1_u64 << 24) as f32;
        unit
    }

    /// A whole number from 0 up to `count`.
    fn below(&mut self, count: i32) -> i32 {
        let count = u64::from(count.max(1).cast_unsigned());
        i32::try_from(self.next() % count).unwrap_or(0)
    }
}

#[cfg(test)]
#[path = "scatter_tests.rs"]
mod tests;
