//! Terrains: which sprite each cell of a painted area draws, from which of
//! its neighbors are the same terrain.
//!
//! Painting ground a tile at a time means choosing the grass-topped one for
//! the top row, the left-edged one for the left column, the corner for the
//! corner, and choosing again every time the shape changes. A terrain names
//! those sprites once, in the order of the neighbor patterns they answer, and
//! a brush or a script paints "grass" and gets the right one.
//!
//! Three layouts are understood. `edges` is sixteen sprites, one for each
//! combination of the four sides: enough for blocky ground. `blob` is the
//! forty-seven that also tell an inside corner from a filled one, for art
//! whose corners show. `corners` is sixteen sprites chosen by the terrain at a
//! tile's four corners rather than by its neighbors, the layout of a Wang or
//! Tiled corner set and of what a tileset generator makes for two terrains.

use serde::{Deserialize, Serialize};

/// Which neighbor patterns a terrain draws differently.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerrainKind {
    /// Sixteen sprites, by which of the four sides are the same terrain:
    /// north 1, east 2, south 4, west 8.
    #[default]
    Edges,
    /// Forty-seven sprites, by which of the eight neighbors are, with a corner
    /// counting only when both sides beside it do. Listed in the order of
    /// their masks: north 1, north-east 2, east 4, south-east 8, south 16,
    /// south-west 32, west 64, north-west 128.
    Blob,
    /// Sixteen sprites, by which of a tile's four corners are the terrain:
    /// north-west 8, north-east 4, south-west 2, south-east 1. Sprite 0, no
    /// corner, is the ground the terrain lies on, and belongs to the terrain
    /// like the rest. It is painted by corner, with `paint_corner`, not by
    /// cell: a cell's sprite holds its corners, and painting one corner
    /// rewrites the up to four tiles that meet there.
    Corners,
}

impl TerrainKind {
    pub const NAMES: [&'static str; 3] = ["edges", "blob", "corners"];

    /// How many sprites a terrain of this kind lists.
    #[must_use]
    pub const fn sprites(self) -> usize {
        match self {
            Self::Edges | Self::Corners => 16,
            Self::Blob => 47,
        }
    }
}

/// One terrain of a sheet.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainDocument {
    #[serde(default)]
    pub kind: TerrainKind,
    /// The sprite for each neighbor pattern, in the kind's order.
    pub sprites: Vec<String>,
}

/// Which of a cell's eight neighbors are the same terrain, clockwise from
/// north: north, north-east, east, south-east, south, south-west, west,
/// north-west.
pub type Neighbors = [bool; 8];

impl TerrainDocument {
    /// The sprite a cell with these neighbors draws.
    #[must_use]
    pub fn sprite_for(&self, neighbors: Neighbors) -> Option<&str> {
        let index = match self.kind {
            TerrainKind::Edges => usize::from(edge_mask(neighbors)),
            TerrainKind::Blob => blob_index(blob_mask(neighbors))?,
            // Chosen by corners, never by neighbors: see `sprite_for_corners`.
            TerrainKind::Corners => return None,
        };
        self.sprites.get(index).map(String::as_str)
    }

    /// The sprite of a corners terrain whose tile has these corners set, as a
    /// number from 0 to 15, or `None` for another kind or a number past 15.
    #[must_use]
    pub fn sprite_for_corners(&self, corners: u8) -> Option<&str> {
        if self.kind != TerrainKind::Corners {
            return None;
        }
        self.sprites.get(usize::from(corners)).map(String::as_str)
    }

    /// Which corners of a corners terrain's tile are set, if `sprite` is one
    /// of its sprites.
    #[must_use]
    pub fn corners_of(&self, sprite: &str) -> Option<u8> {
        if self.kind != TerrainKind::Corners {
            return None;
        }
        self.sprites
            .iter()
            .position(|own| own == sprite)
            .and_then(|index| u8::try_from(index).ok())
    }

    /// Whether `sprite` is one of this terrain's, which is what makes a cell
    /// part of it.
    #[must_use]
    pub fn owns(&self, sprite: &str) -> bool {
        self.sprites.iter().any(|own| own == sprite)
    }
}

/// The four sides as a number from 0 to 15.
#[must_use]
pub fn edge_mask(neighbors: Neighbors) -> u8 {
    [(0, 1), (2, 2), (4, 4), (6, 8)]
        .into_iter()
        .filter(|(side, _)| neighbors[*side])
        .map(|(_, bit)| bit)
        .sum()
}

/// All eight as a number, a corner kept only when both its sides are set:
/// a diagonal neighbor past an open side changes nothing a tile can show.
#[must_use]
pub fn blob_mask(neighbors: Neighbors) -> u8 {
    let mut mask = 0_u8;
    for (index, &set) in neighbors.iter().enumerate() {
        let corner = index % 2 == 1;
        let counts = set && (!corner || (neighbors[index - 1] && neighbors[(index + 1) % 8]));
        if counts {
            mask |= 1 << index;
        }
    }
    mask
}

/// Where a reduced blob mask falls among the forty-seven, in mask order.
#[must_use]
pub fn blob_index(mask: u8) -> Option<usize> {
    blob_masks().iter().position(|known| *known == mask)
}

/// The forty-seven masks a blob terrain lists, ascending.
#[must_use]
pub fn blob_masks() -> Vec<u8> {
    (0..=255_u8)
        .filter(|mask| {
            let neighbors: Neighbors = std::array::from_fn(|bit| mask & (1 << bit) != 0);
            blob_mask(neighbors) == *mask
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{TerrainDocument, TerrainKind, blob_mask, blob_masks, edge_mask};

    #[test]
    fn a_corners_terrain_is_chosen_by_corners_alone() {
        let corners = TerrainDocument {
            kind: TerrainKind::Corners,
            sprites: (0..16).map(|mask| format!("c{mask}")).collect(),
        };
        assert_eq!(corners.sprite_for([true; 8]), None, "neighbors say nothing");
        assert_eq!(corners.sprite_for_corners(0b1001), Some("c9"));
        assert_eq!(corners.sprite_for_corners(16), None);
        assert_eq!(corners.corners_of("c12"), Some(12));
        assert_eq!(corners.corners_of("other"), None);
        let edges = TerrainDocument {
            kind: TerrainKind::Edges,
            sprites: corners.sprites.clone(),
        };
        assert_eq!(
            edges.corners_of("c3"),
            None,
            "only a corners terrain has corners"
        );
    }

    #[test]
    fn there_are_forty_seven_blob_masks() {
        assert_eq!(blob_masks().len(), 47);
    }

    /// A diagonal neighbor only counts when both sides beside it do.
    #[test]
    fn a_lone_corner_is_ignored() {
        let only_north_east = [false, true, false, false, false, false, false, false];
        assert_eq!(blob_mask(only_north_east), 0);
        let closed = [true, true, true, false, false, false, false, false];
        assert_eq!(blob_mask(closed), 0b111);
    }

    #[test]
    fn the_sides_make_the_edge_mask() {
        let north_and_west = [true, false, false, false, false, false, true, false];
        assert_eq!(edge_mask(north_and_west), 9);
    }

    #[test]
    fn a_cell_draws_the_sprite_for_its_pattern() {
        let terrain = TerrainDocument {
            kind: TerrainKind::Edges,
            sprites: (0..16).map(|index| format!("g{index}")).collect(),
        };
        let surrounded = [true; 8];
        assert_eq!(terrain.sprite_for(surrounded), Some("g15"));
        assert!(terrain.owns("g3") && !terrain.owns("stone"));
    }
}
