//! Painting a terrain onto a tilemap: the cell, and every neighbor whose
//! picture depends on it.
//!
//! A terrain is the sheet's to declare (`sindri_core::TerrainDocument`); this
//! is where a brush stroke or a script's `Grid.set_terrain` becomes cells. It
//! works on the stored payload, which is what both of them hold, and writes
//! back only the palette and the tiles.

use std::collections::BTreeMap;

use serde_json::Value;
use sindri_core::{AssetId, Neighbors, TerrainDocument, TerrainKind, sheet_id_for};
use thiserror::Error;

use crate::TilemapComponent;

/// Every sheet's terrains, by the sheet's asset ID.
///
/// Keyed by sheet rather than texture because a tilemap names its texture and
/// the sheet beside it is found by the one naming rule, whatever the image's
/// extension.
#[derive(Clone, Debug, Default)]
pub struct TerrainBindings {
    bound: BTreeMap<String, BTreeMap<String, TerrainDocument>>,
}

impl TerrainBindings {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a sheet's terrains, replacing what it had.
    pub fn bind(&mut self, sheet: impl Into<String>, terrains: BTreeMap<String, TerrainDocument>) {
        let sheet = sheet.into();
        if terrains.is_empty() {
            self.bound.remove(&sheet);
        } else {
            self.bound.insert(sheet, terrains);
        }
    }

    /// The terrains of the sheet that slices `texture`.
    #[must_use]
    pub fn for_texture(&self, texture: &str) -> Option<&BTreeMap<String, TerrainDocument>> {
        let sheet = sheet_id_for(&AssetId::new(texture).ok()?)?;
        self.bound.get(sheet.as_str())
    }
}

/// Why a terrain could not be painted.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum TerrainPaintError {
    #[error("the tilemap does not read: {0}")]
    Map(String),
    #[error("cell {column},{row} is outside a {columns}x{rows} map")]
    Outside {
        column: u32,
        row: u32,
        columns: u32,
        rows: u32,
    },
    #[error("the tilemap's sheet has no terrain called `{0}`")]
    Unknown(String),
    #[error("the tilemap holds the wrong number of cells; resize it first")]
    Cells,
    #[error("terrain `{0}` is painted by corner, not by cell")]
    ByCorner(String),
    #[error("terrain `{0}` is painted by cell, not by corner")]
    ByCell(String),
}

/// Paints `terrain` into a cell, or empties it with `None`, and redraws the
/// cell and its eight neighbors from the patterns they now make. Answers
/// whether anything changed.
///
/// # Errors
/// When the payload is not a tilemap, the cell is off it, or the terrain is
/// not one of `terrains`.
pub fn paint_terrain(
    payload: &mut Value,
    terrains: &BTreeMap<String, TerrainDocument>,
    column: u32,
    row: u32,
    terrain: Option<&str>,
) -> Result<bool, TerrainPaintError> {
    let mut map: TilemapComponent = serde_json::from_value(payload.clone())
        .map_err(|error| TerrainPaintError::Map(error.to_string()))?;
    if column >= map.columns || row >= map.rows {
        return Err(TerrainPaintError::Outside {
            column,
            row,
            columns: map.columns,
            rows: map.rows,
        });
    }
    if map.tiles.len() != map.expected_cells() {
        return Err(TerrainPaintError::Cells);
    }
    let before = (map.palette.clone(), map.tiles.clone());
    let cell = cell_index(&map, column, row);
    match terrain {
        Some(name) => {
            let chosen = terrains
                .get(name)
                .ok_or_else(|| TerrainPaintError::Unknown(name.to_owned()))?;
            if chosen.kind == TerrainKind::Corners {
                return Err(TerrainPaintError::ByCorner(name.to_owned()));
            }
            // Its own sprite is decided by its neighbors alone, so it is
            // written straight away rather than through a placeholder that
            // would leave a sprite in the palette nothing draws.
            let neighbors = pattern(&map, terrains, name, column, row);
            let sprite = chosen
                .sprite_for(neighbors)
                .ok_or_else(|| TerrainPaintError::Unknown(name.to_owned()))?;
            map.tiles[cell] = Some(palette_slot(&mut map.palette, sprite));
        }
        None => map.tiles[cell] = None,
    }
    let columns = i64::from(map.columns);
    let rows = i64::from(map.rows);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (x, y) = (i64::from(column) + dx, i64::from(row) + dy);
            if (0..columns).contains(&x) && (0..rows).contains(&y) {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                redraw(&mut map, terrains, x as u32, y as u32);
            }
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

/// Which terrain a cell belongs to: the first that owns its sprite.
pub(crate) fn terrain_at<'a>(
    map: &TilemapComponent,
    terrains: &'a BTreeMap<String, TerrainDocument>,
    column: i64,
    row: i64,
) -> Option<&'a str> {
    if !(0..i64::from(map.columns)).contains(&column) || !(0..i64::from(map.rows)).contains(&row) {
        return None;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let tile = map.tiles[cell_index(map, column as u32, row as u32)]?;
    let sprite = map.palette.get(tile as usize)?;
    terrains
        .iter()
        .find(|(_, terrain)| terrain.owns(sprite))
        .map(|(name, _)| name.as_str())
}

/// Gives a terrain cell the sprite its neighbors call for; leaves any other
/// cell alone.
fn redraw(
    map: &mut TilemapComponent,
    terrains: &BTreeMap<String, TerrainDocument>,
    column: u32,
    row: u32,
) {
    let Some(name) = terrain_at(map, terrains, i64::from(column), i64::from(row)) else {
        return;
    };
    let neighbors = pattern(map, terrains, name, column, row);
    if let Some(sprite) = terrains[name].sprite_for(neighbors) {
        let slot = palette_slot(&mut map.palette, sprite);
        let cell = cell_index(map, column, row);
        map.tiles[cell] = Some(slot);
    }
}

/// Which of a cell's neighbors are the terrain `name`.
fn pattern(
    map: &TilemapComponent,
    terrains: &BTreeMap<String, TerrainDocument>,
    name: &str,
    column: u32,
    row: u32,
) -> Neighbors {
    let (x, y) = (i64::from(column), i64::from(row));
    // Clockwise from north; rows count down the map, so north is one less.
    let around = [
        (0, -1),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
    ];
    // Past the map's edge counts as more of the same: ground painted to the
    // edge carries on out of the picture rather than ending in a rim there.
    let (columns, rows) = (i64::from(map.columns), i64::from(map.rows));
    around.map(|(dx, dy)| {
        let (nx, ny) = (x + dx, y + dy);
        let outside = !(0..columns).contains(&nx) || !(0..rows).contains(&ny);
        outside || terrain_at(map, terrains, nx, ny) == Some(name)
    })
}

pub(crate) fn palette_slot(palette: &mut Vec<String>, sprite: &str) -> u32 {
    let index = palette
        .iter()
        .position(|known| known == sprite)
        .unwrap_or_else(|| {
            palette.push(sprite.to_owned());
            palette.len() - 1
        });
    u32::try_from(index).unwrap_or(u32::MAX)
}

pub(crate) fn cell_index(map: &TilemapComponent, column: u32, row: u32) -> usize {
    row as usize * map.columns as usize + column as usize
}

/// The terrains a set of sheets declares, for hosts that hold sheets by ID.
pub fn terrain_bindings<'a>(
    sheets: impl IntoIterator<Item = (&'a str, &'a sindri_core::SpriteSheetDocument)>,
) -> TerrainBindings {
    let mut bindings = TerrainBindings::new();
    for (id, sheet) in sheets {
        bindings.bind(id, sheet.terrains.clone());
    }
    bindings
}

#[cfg(test)]
#[path = "terrain_paint_tests.rs"]
mod tests;
