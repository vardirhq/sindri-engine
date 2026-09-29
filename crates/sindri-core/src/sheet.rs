//! A sliced image: named parts of one texture, described beside it.
//!
//! The problem this solves is duplication. Before it, three components each
//! said how a sheet was cut — a sprite carried a raw rect, an animation carried
//! a grid and cell numbers, a tilemap carried a second grid and more cell
//! numbers — so the same image used twice declared its layout twice and nothing
//! made the two agree. A sheet is a property of the image, not of whoever draws
//! it, so it belongs beside the image and is said once.
//!
//! A sheet document sits at a derived ID: `textures/tiles.png` is sliced by
//! `textures/tiles.sheet`. Derived rather than declared, because a scene
//! naming its sheets would be a fourth place that can disagree.
//!
//! Nothing here knows what a `UvRect` is — that is `sindri-render`'s, and this
//! crate does not depend on it. Rects are stored as they are authored and
//! checked where they are used, which is the same arrangement `SceneDocument`
//! has with the components it carries.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::AssetId;

#[cfg(test)]
mod tests;

/// The version this runtime writes and understands.
pub const SHEET_FORMAT_VERSION: u32 = 1;

/// The suffix that turns a texture's ID into its sheet's ID.
pub const SHEET_SUFFIX: &str = ".sheet";
/// Legacy suffix accepted while projects migrate to the native extension.
pub const LEGACY_SHEET_SUFFIX: &str = ".sheet.json";

/// Where a sprite meets the ground, as a fraction of its frame.
///
/// A quad is drawn centred on the entity's position, so without this the middle
/// of the picture is what lands on the tile. That is right for a floating orb
/// and wrong for anything that stands: a character drawn in the middle of its
/// frame is drawn half a body low, standing in the tile in front of its own.
///
/// It belongs to the image for the same reason the slice does — it is a fact
/// about the picture, not about whoever draws it, so it is said once beside the
/// image instead of by every scene that uses it.
///
/// `Center` is the default because that is what a quad already does, what the
/// baker's frames are padded to, and what every engine with a sprite pivot
/// defaults to. Baked art declares it anyway rather than leaning on the
/// default, so a sheet describes itself.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SpriteAnchor {
    /// The middle of the frame. What a quad does on its own.
    #[default]
    Center,
    /// The middle of the frame's bottom edge, which is where a sprite drawn
    /// standing on the floor of its own frame touches the ground.
    Bottom,
    /// An exact point, as fractions of the frame from its top-left corner.
    ///
    /// Fractions rather than pixels so a sheet needs no image size to be
    /// understood, and so the anchor survives the art being redrawn at another
    /// resolution.
    Fraction([f32; 2]),
}

impl SpriteAnchor {
    /// Where the anchor sits, as fractions of the frame from its top-left.
    #[must_use]
    pub const fn fraction(self) -> [f32; 2] {
        match self {
            Self::Center => [0.5, 0.5],
            Self::Bottom => [0.5, 1.0],
            Self::Fraction(fraction) => fraction,
        }
    }

    /// How far to move the drawn quad so the anchor lands on the entity.
    ///
    /// In the quad's own space, where it spans -0.5 to 0.5 and y counts up
    /// from the bottom, this is simply the difference from the centre.
    #[must_use]
    pub fn quad_offset(self) -> [f32; 2] {
        let [x, y] = self.fraction();
        [0.5 - x, y - 0.5]
    }

    fn validate(self) -> Result<(), SheetError> {
        let [x, y] = self.fraction();
        if !x.is_finite() || !y.is_finite() || !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
            return Err(SheetError::Anchor { x, y });
        }
        Ok(())
    }
}

impl Serialize for SpriteAnchor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Center => serializer.serialize_str("center"),
            Self::Bottom => serializer.serialize_str("bottom"),
            Self::Fraction(fraction) => fraction.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for SpriteAnchor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Name(String),
            Fraction([f32; 2]),
        }

        match Repr::deserialize(deserializer)? {
            Repr::Name(name) if name == "center" => Ok(Self::Center),
            Repr::Name(name) if name == "bottom" => Ok(Self::Bottom),
            Repr::Name(name) => Err(serde::de::Error::custom(format!("unknown sprite anchor `{name}`"))),
            Repr::Fraction(fraction) => Ok(Self::Fraction(fraction)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpriteSheetDocument {
    pub format_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<SpriteAnchor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tile_overhang_ratio: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<SheetGrid>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sprites: BTreeMap<String, [f32; 4]>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub editor: BTreeMap<String, Value>,
}

impl Default for SpriteSheetDocument {
    fn default() -> Self {
        Self {
            format_version: SHEET_FORMAT_VERSION,
            anchor: None,
            tile_overhang_ratio: None,
            grid: None,
            sprites: BTreeMap::new(),
            editor: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SheetGrid {
    pub columns: u32,
    pub rows: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<[u32; 2]>,
    #[serde(default = "zero_pair", skip_serializing_if = "is_zero")]
    pub margin: [u32; 2],
    #[serde(default = "zero_pair", skip_serializing_if = "is_zero")]
    pub spacing: [u32; 2],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
}

const fn zero_pair() -> [u32; 2] { [0, 0] }
fn is_zero(value: &[u32; 2]) -> bool { *value == [0, 0] }

impl SheetGrid {
    #[must_use]
    pub fn edge_to_edge(columns: u32, rows: u32) -> Self {
        Self { columns, rows, size: None, margin: [0, 0], spacing: [0, 0], names: Vec::new() }
    }

    #[must_use]
    pub const fn cells(&self) -> u32 { self.columns.saturating_mul(self.rows) }

    #[must_use]
    pub fn name_of(&self, index: u32) -> String {
        self.names.get(index as usize).cloned().unwrap_or_else(|| index.to_string())
    }

    fn rect_of(&self, index: u32) -> Option<[f32; 4]> {
        if self.columns == 0 || self.rows == 0 || index >= self.cells() { return None; }
        let column = index % self.columns;
        let row = index / self.columns;
        if let Some([image_width, image_height]) = self.size {
            let [x, width] = measured_axis(image_width, self.columns, self.margin[0], self.spacing[0], column)?;
            let [y, height] = measured_axis(image_height, self.rows, self.margin[1], self.spacing[1], row)?;
            return Some([
                x as f32 / image_width as f32,
                y as f32 / image_height as f32,
                width as f32 / image_width as f32,
                height as f32 / image_height as f32,
            ]);
        }
        Some([
            column as f32 / self.columns as f32,
            row as f32 / self.rows as f32,
            1.0 / self.columns as f32,
            1.0 / self.rows as f32,
        ])
    }
}

fn measured_axis(image: u32, cells: u32, margin: u32, spacing: u32, index: u32) -> Option<[u32; 2]> {
    let gutters = spacing.checked_mul(cells.saturating_sub(1))?;
    let borders = margin.checked_mul(2)?;
    let occupied = gutters.checked_add(borders)?;
    let remaining = image.checked_sub(occupied)?;
    let cell = remaining.checked_div(cells)?;
    if cell == 0 { return None; }
    let start = margin.checked_add(index.checked_mul(cell.checked_add(spacing)?)?)?;
    Some([start, cell])
}

impl SpriteSheetDocument {
    /// A uniform sheet of `columns` by `rows`, with cells named by index.
    #[must_use]
    pub fn from_grid(columns: u32, rows: u32) -> Self {
        Self {
            format_version: SHEET_FORMAT_VERSION,
            anchor: None,
            tile_overhang_ratio: None,
            grid: Some(SheetGrid::edge_to_edge(columns, rows)),
            sprites: BTreeMap::new(),
            editor: BTreeMap::new(),
        }
    }

    /// Parses a sheet, rejecting a version this runtime does not write.
    pub fn from_json(json: &str) -> Result<Self, SheetError> {
        let document: Self = serde_json::from_str(json).map_err(|error| SheetError::Json { message: error.to_string() })?;
        document.validate()?;
        Ok(document)
    }

    pub fn to_canonical_json(&self) -> Result<String, SheetError> {
        self.validate()?;
        let mut json = serde_json::to_string_pretty(self).map_err(|error| SheetError::Json { message: error.to_string() })?;
        json.push('\n');
        Ok(json)
    }

    pub fn rects(&self) -> Result<BTreeMap<String, [f32; 4]>, SheetError> {
        self.anchor.unwrap_or_default().validate()?;
        let mut rects = BTreeMap::new();
        if let Some(grid) = &self.grid {
            if grid.columns == 0 || grid.rows == 0 { return Err(SheetError::EmptyGrid { columns: grid.columns, rows: grid.rows }); }
            if grid.names.len() > grid.cells() as usize { return Err(SheetError::TooManyNames { names: grid.names.len(), cells: grid.cells() as usize }); }
            if (!is_zero(&grid.margin) || !is_zero(&grid.spacing)) && grid.size.is_none() { return Err(SheetError::MeasuredWithoutSize); }
            for index in 0..grid.cells() {
                let name = grid.name_of(index);
                let rect = grid.rect_of(index).ok_or(SheetError::CellDoesNotFit { index })?;
                if rects.insert(name.clone(), rect).is_some() { return Err(SheetError::DuplicateName(name)); }
            }
        }
        for (name, rect) in &self.sprites {
            if rects.insert(name.clone(), *rect).is_some() { return Err(SheetError::DuplicateName(name.clone())); }
        }
        if rects.is_empty() { return Err(SheetError::Empty); }
        Ok(rects)
    }

    fn validate(&self) -> Result<(), SheetError> {
        if self.format_version != SHEET_FORMAT_VERSION {
            return Err(SheetError::UnsupportedVersion { found: self.format_version, supported: SHEET_FORMAT_VERSION });
        }
        if let Some(ratio) = self.tile_overhang_ratio {
            if !ratio.is_finite() || ratio < 0.0 { return Err(SheetError::TileOverhangRatio(ratio)); }
        }
        self.rects().map(|_| ())
    }
}

/// The sheet that slices `texture`, by the one naming rule.
///
/// `textures/tiles.png` is sliced by `textures/tiles.sheet`. A texture whose ID
/// already ends in either the native or legacy suffix is not a texture, and
/// gets `None` rather than a sheet of a sheet.
#[must_use]
pub fn sheet_id_for(texture: &AssetId) -> Option<AssetId> {
    let path = texture.as_str();
    if path.ends_with(SHEET_SUFFIX) || path.ends_with(LEGACY_SHEET_SUFFIX) { return None; }
    let stem = path.rsplit_once('.').map_or(path, |(stem, _)| stem);
    AssetId::new(format!("{stem}{SHEET_SUFFIX}")).ok()
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum SheetError {
    #[error("sheet format version {found} is not supported (this runtime writes {supported})")]
    UnsupportedVersion { found: u32, supported: u32 },
    #[error("a sheet must name at least one sprite")]
    Empty,
    #[error("a sheet grid of {columns}x{rows} has no cells")]
    EmptyGrid { columns: u32, rows: u32 },
    #[error("the grid names {names} cells but only has {cells}")]
    TooManyNames { names: usize, cells: usize },
    #[error("two sprites in one sheet are both called `{0}`")]
    DuplicateName(String),
    #[error("a grid with a margin or spacing measures in pixels, so it must record the size of the image it was cut against")]
    MeasuredWithoutSize,
    #[error("cell {index} does not fit the grid it was cut from")]
    CellDoesNotFit { index: u32 },
    #[error("an anchor sits at ({x}, {y}), which is not a point on the frame")]
    Anchor { x: f32, y: f32 },
    #[error("tile overhang ratio must be finite and non-negative, got {0}")]
    TileOverhangRatio(f32),
    #[error("sheet is not valid json: {message}")]
    Json { message: String },
}

#[cfg(test)]
mod margin_tests {
    use super::{SHEET_FORMAT_VERSION, SheetError, SheetGrid, SpriteSheetDocument};

    fn packed() -> SpriteSheetDocument {
        SpriteSheetDocument {
            format_version: SHEET_FORMAT_VERSION,
            grid: Some(SheetGrid { columns: 16, rows: 16, size: Some([512, 512]), margin: [2, 2], spacing: [4, 4], names: Vec::new() }),
            ..SpriteSheetDocument::default()
        }
    }

    #[test]
    fn a_gutter_is_left_out_of_every_cell() {
        let sheet = packed();
        let rects = sheet.rects().expect("a packed grid slices");
        let close = |left: f32, right: f32| (left - right).abs() < 1.0e-5;
        let first = rects["0"];
        assert!(close(first[0], 2.0 / 512.0));
        assert!(close(first[2], 28.0 / 512.0));
        let second = rects["1"];
        assert!(close(second[0], (2.0 + 28.0 + 4.0) / 512.0));
        let last = rects["255"];
        assert!(close(last[0] + last[2], (512.0 - 2.0) / 512.0));
    }

    #[test]
    fn an_edge_to_edge_grid_needs_no_size() {
        let sheet = SpriteSheetDocument::from_grid(4, 1);
        assert!(sheet.grid.as_ref().expect("a grid").size.is_none());
        assert!(sheet.rects().is_ok());
    }

    #[test]
    fn a_measured_grid_without_a_size_is_refused() {
        let mut sheet = packed();
        sheet.grid.as_mut().expect("a grid").size = None;
        assert_eq!(sheet.rects(), Err(SheetError::MeasuredWithoutSize));
    }

    #[test]
    fn spacing_that_leaves_no_room_is_refused() {
        let mut sheet = packed();
        sheet.grid.as_mut().expect("a grid").spacing = [64, 64];
        assert!(matches!(sheet.rects(), Err(SheetError::CellDoesNotFit { .. })));
    }
}
