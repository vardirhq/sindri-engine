//! Painting a corners terrain: one corner, and the tiles that meet there.
//!
//! A corners terrain's sprite says which of the tile's four corners are the
//! terrain, so there is no cell to paint, only the corners between cells. A
//! corner is addressed by the grid line crossing it: column 0 to `columns`,
//! row 0 to `rows`, so a map of 3 by 2 tiles has 4 by 3 corners. Setting one
//! rewrites the tiles on its four sides, each reading its own corners back
//! from the sprite it already shows.

use std::collections::BTreeMap;

use serde_json::Value;
use sindri_core::{TerrainDocument, TerrainKind};

use crate::TilemapComponent;
use crate::terrain_paint::{TerrainPaintError, cell_index, palette_slot, terrain_at};

/// The four tiles meeting at a corner, with the corner of each it is:
/// (tile column offset, tile row offset, that tile's corner bit). The corner
/// is the tile's south-east where the tile lies north-west of it, and so on.
const AROUND: [(i64, i64, u8); 4] = [(-1, -1, 1), (0, -1, 2), (-1, 0, 4), (0, 0, 8)];

/// Sets or clears the corner at `column`, `row` for the corners terrain
/// `terrain`, and redraws the tiles that meet there. Answers whether
/// anything changed.
///
/// A tile that is empty reads as having no corners; a tile that shows any
/// other sprite, another terrain's or none's, is left alone. Clearing a corner on an empty tile writes
/// nothing.
///
/// # Errors
/// When the payload is not a tilemap, the corner is off its grid lines, or
/// `terrain` is missing or not a corners terrain.
pub fn paint_corner(
    payload: &mut Value,
    terrains: &BTreeMap<String, TerrainDocument>,
    column: u32,
    row: u32,
    terrain: &str,
    set: bool,
) -> Result<bool, TerrainPaintError> {
    let mut map: TilemapComponent = serde_json::from_value(payload.clone())
        .map_err(|error| TerrainPaintError::Map(error.to_string()))?;
    if column > map.columns || row > map.rows {
        return Err(TerrainPaintError::Outside {
            column,
            row,
            columns: map.columns + 1,
            rows: map.rows + 1,
        });
    }
    if map.tiles.len() != map.expected_cells() {
        return Err(TerrainPaintError::Cells);
    }
    let chosen = terrains
        .get(terrain)
        .ok_or_else(|| TerrainPaintError::Unknown(terrain.to_owned()))?;
    if chosen.kind != TerrainKind::Corners {
        return Err(TerrainPaintError::ByCell(terrain.to_owned()));
    }
    let before = (map.palette.clone(), map.tiles.clone());
    for (dx, dy, bit) in AROUND {
        let (x, y) = (i64::from(column) + dx, i64::from(row) + dy);
        if !(0..i64::from(map.columns)).contains(&x) || !(0..i64::from(map.rows)).contains(&y) {
            continue;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let cell = cell_index(&map, x as u32, y as u32);
        let owner = terrain_at(&map, terrains, x, y);
        // Only an empty tile or one of this terrain's own is ours to rewrite.
        if owner != Some(terrain) && !(owner.is_none() && map.tiles[cell].is_none()) {
            continue;
        }
        let current = map.tiles[cell]
            .and_then(|tile| map.palette.get(tile as usize))
            .and_then(|sprite| chosen.corners_of(sprite))
            .unwrap_or(0);
        let corners = if set { current | bit } else { current & !bit };
        if owner.is_none() && corners == 0 {
            continue;
        }
        if let Some(sprite) = chosen.sprite_for_corners(corners) {
            let slot = palette_slot(&mut map.palette, sprite);
            map.tiles[cell] = Some(slot);
        }
    }
    if (map.palette.clone(), map.tiles.clone()) == before {
        return Ok(false);
    }
    let object = payload
        .as_object_mut()
        .ok_or_else(|| TerrainPaintError::Map("not an object".to_owned()))?;
    object.insert(
        "palette".to_owned(),
        Value::from(map.palette.iter().map(String::as_str).collect::<Vec<_>>()),
    );
    let tiles = map
        .tiles
        .iter()
        .map(|tile| tile.map_or(Value::Null, Value::from));
    object.insert("tiles".to_owned(), Value::Array(tiles.collect()));
    Ok(true)
}

#[cfg(test)]
#[path = "terrain_corners_tests.rs"]
mod tests;
