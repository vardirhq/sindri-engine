//! Reusable semantic tiles and the baked faces that represent them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

mod shape;
mod validate;

pub use shape::TileBox;
use validate::validate_visual;

pub const TILESET_FORMAT_VERSION: u32 = 1;
pub const TILESET_SUFFIX: &str = ".tileset";
pub const LEGACY_TILESET_SUFFIX: &str = ".tileset.json";

/// The block set the engine ships: grass, dirt, stone, water, lava and the
/// rest. Its bytes live in `sindri-assets`; the name lives here so a scene can
/// default to it without depending on where its art comes from.
pub const BUILTIN_BLOCKS: &str = "builtin:blocks";

/// One face of an axis-aligned logical tile cell.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TileFace {
    Bottom,
    North,
    West,
    East,
    South,
    Top,
}

impl TileFace {
    /// Every face, in the order a cube's sides are reasoned about.
    pub const ALL: [Self; 6] = [
        Self::Top,
        Self::Bottom,
        Self::North,
        Self::South,
        Self::East,
        Self::West,
    ];

    /// The face on the other side of the same block.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Bottom => Self::Top,
            Self::North => Self::South,
            Self::West => Self::East,
            Self::East => Self::West,
            Self::South => Self::North,
            Self::Top => Self::Bottom,
        }
    }

    /// The cell whose presence may hide this face.
    #[must_use]
    pub const fn neighbour_offset(self) -> [i32; 3] {
        match self {
            Self::Bottom => [0, 0, -1],
            Self::North => [0, -1, 0],
            Self::West => [-1, 0, 0],
            Self::East => [1, 0, 0],
            Self::South => [0, 1, 0],
            Self::Top => [0, 0, 1],
        }
    }
}

/// A face that moves: water rippling, lava churning.
///
/// Frames rather than a scrolling texture, because a ripple drawn by hand
/// reads as water in a way a texture sliding sideways never does. Every frame
/// names a sprite of the same size on the same texture as the face's own, so
/// a renderer can move between them by shifting where it reads rather than
/// by rebuilding what it draws.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FaceAnimation {
    /// The frames after the face's own sprite, in order.
    pub frames: Vec<String>,
    /// Frames per second.
    pub fps: f32,
}

/// One pre-rendered 2D face.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TileFaceVisual {
    /// Texture or named sheet sprite, such as `blocks.png#grass_top`.
    pub sprite: String,
    /// Quad size in the tile grid's local world units.
    pub size: [f32; 2],
    /// Quad-centre offset from the projected cell centre.
    #[serde(default, skip_serializing_if = "is_zero_vec")]
    pub offset: [f32; 2],
    /// Further frames the face cycles through, starting from `sprite`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation: Option<FaceAnimation>,
}

/// Optional visuals for every face a view may expose.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct TileFaces {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom: Option<TileFaceVisual>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub north: Option<TileFaceVisual>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub west: Option<TileFaceVisual>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub east: Option<TileFaceVisual>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub south: Option<TileFaceVisual>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<TileFaceVisual>,
}

impl TileFaces {
    pub fn iter(&self) -> impl Iterator<Item = (TileFace, &TileFaceVisual)> {
        [
            (TileFace::Bottom, self.bottom.as_ref()),
            (TileFace::North, self.north.as_ref()),
            (TileFace::West, self.west.as_ref()),
            (TileFace::East, self.east.as_ref()),
            (TileFace::South, self.south.as_ref()),
            (TileFace::Top, self.top.as_ref()),
        ]
        .into_iter()
        .filter_map(|(face, visual)| visual.map(|visual| (face, visual)))
    }

    /// What to draw on a face, falling back to the face opposite it.
    #[must_use]
    pub fn resolved_in<'a>(
        &'a self,
        face: TileFace,
        covered: Option<&'a TileFaces>,
    ) -> Option<(TileFace, &'a TileFaceVisual)> {
        covered
            .and_then(|buried| buried.resolved(face))
            .or_else(|| self.resolved(face))
    }

    #[must_use]
    pub fn resolved(&self, face: TileFace) -> Option<(TileFace, &TileFaceVisual)> {
        if let Some(visual) = self.get(face) {
            return Some((face, visual));
        }
        let opposite = face.opposite();
        self.get(opposite).map(|visual| (opposite, visual))
    }

    #[must_use]
    pub fn get(&self, face: TileFace) -> Option<&TileFaceVisual> {
        match face {
            TileFace::Bottom => self.bottom.as_ref(),
            TileFace::North => self.north.as_ref(),
            TileFace::West => self.west.as_ref(),
            TileFace::East => self.east.as_ref(),
            TileFace::South => self.south.as_ref(),
            TileFace::Top => self.top.as_ref(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TileVariant {
    pub faces: TileFaces,
    #[serde(default = "one_weight", skip_serializing_if = "is_one")]
    pub weight: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TileDefinition {
    pub faces: TileFaces,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub covered: Option<TileFaces>,
    #[serde(default = "full_height", skip_serializing_if = "is_full_height")]
    pub height: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extent: Option<TileBox>,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub occludes: bool,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub supports: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<TileVariant>,
    #[serde(default = "one_weight", skip_serializing_if = "is_one")]
    pub weight: u32,
    #[serde(default = "yes", alias = "solid", skip_serializing_if = "is_true")]
    pub walkable: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub glow: f32,
}

impl TileDefinition {
    #[must_use]
    pub fn bounds(&self) -> TileBox {
        self.extent
            .unwrap_or_else(|| TileBox::from_height(self.height))
    }

    #[must_use]
    pub fn faces_at(&self, seed: u64, coord: [i32; 3], tile: &str) -> &TileFaces {
        if self.variants.is_empty() {
            return &self.faces;
        }
        let mut weights = Vec::with_capacity(self.variants.len() + 1);
        weights.push(self.weight);
        weights.extend(self.variants.iter().map(|variant| variant.weight));
        #[allow(clippy::cast_possible_wrap)]
        let hash = crate::stable_hash_with(
            &[
                i64::from(coord[0]),
                i64::from(coord[1]),
                i64::from(coord[2]),
                seed as i64,
            ],
            tile,
        );
        match crate::weighted_index(hash, &weights) {
            Some(0) | None => &self.faces,
            Some(index) => &self.variants[index - 1].faces,
        }
    }

    pub fn all_faces(&self) -> impl Iterator<Item = &TileFaces> {
        std::iter::once(&self.faces).chain(self.variants.iter().map(|variant| &variant.faces))
    }

    #[must_use]
    pub fn fills_cell(&self) -> bool {
        self.bounds().is_full()
    }

    #[must_use]
    pub fn hides_side_of(&self, height: f32) -> bool {
        let shape = self.bounds();
        self.occludes
            && shape.min == [0.0, 0.0, 0.0]
            && shape.max[0] >= 1.0
            && shape.max[2] >= 1.0
            && shape.top() >= height
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TileSetDocument {
    pub format_version: u32,
    pub tiles: BTreeMap<String, TileDefinition>,
}

impl TileSetDocument {
    pub fn from_json(json: &str) -> Result<Self, TileSetError> {
        let document: Self = serde_json::from_str(json).map_err(|error| TileSetError::Json {
            message: error.to_string(),
        })?;
        document.validate()?;
        Ok(document)
    }

    pub fn validate(&self) -> Result<(), TileSetError> {
        if self.format_version != TILESET_FORMAT_VERSION {
            return Err(TileSetError::UnsupportedVersion {
                found: self.format_version,
                supported: TILESET_FORMAT_VERSION,
            });
        }
        if self.tiles.is_empty() {
            return Err(TileSetError::Empty);
        }
        for (tile, definition) in &self.tiles {
            if tile.trim().is_empty() {
                return Err(TileSetError::EmptyTileId);
            }
            if !definition.height.is_finite() || definition.height <= 0.0 || definition.height > 1.0 {
                return Err(TileSetError::InvalidHeight { tile: tile.clone(), height: definition.height });
            }
            if definition.extent.is_some() && !is_full_height(&definition.height) {
                return Err(TileSetError::HeightAndExtent(tile.clone()));
            }
            if let Some(extent) = definition.extent {
                let sane = |axis: usize| {
                    let (low, high) = (extent.min[axis], extent.max[axis]);
                    low.is_finite() && high.is_finite() && (0.0..=1.0).contains(&low) && (0.0..=1.0).contains(&high) && high > low
                };
                if !(0..3).all(sane) {
                    return Err(TileSetError::InvalidExtent { tile: tile.clone(), extent });
                }
            }
            if !(definition.glow.is_finite() && definition.glow >= 0.0) {
                return Err(TileSetError::InvalidGlow(tile.clone()));
            }
            if definition.walkable && !definition.supports {
                return Err(TileSetError::WalkableWithoutSupport(tile.clone()));
            }
            for faces in definition.all_faces() {
                if faces.iter().next().is_none() {
                    return Err(TileSetError::TileWithoutFaces(tile.clone()));
                }
                for (face, visual) in faces.iter() {
                    validate_visual(tile, face, visual)?;
                }
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn tile(&self, id: &str) -> Option<&TileDefinition> {
        self.tiles.get(id)
    }
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum TileSetError {
    #[error("tile set JSON is not valid: {message}")]
    Json { message: String },
    #[error("tile set format version {found} is not supported; expected {supported}")]
    UnsupportedVersion { found: u32, supported: u32 },
    #[error("a tile set must define at least one tile")]
    Empty,
    #[error("a tile ID cannot be empty")]
    EmptyTileId,
    #[error("tile `{0}` is walkable but supports nothing; standing on a tile rests on it")]
    WalkableWithoutSupport(String),
    #[error("tile `{0}` sets both `height` and `extent`; a box already says how tall it is")]
    HeightAndExtent(String),
    #[error("tile `{tile}` has an extent {extent:?} that is not inside its cell with a positive size")]
    InvalidExtent { tile: String, extent: TileBox },
    #[error("tile `{0}` does not define any face visuals")]
    TileWithoutFaces(String),
    #[error("tile `{tile}` has an invalid {face:?} sprite reference `{sprite}`")]
    InvalidSprite { tile: String, face: TileFace, sprite: String },
    #[error("tile `{tile}` has a non-finite or non-positive {face:?} visual size")]
    InvalidSize { tile: String, face: TileFace },
    #[error("tile `{tile}` has a non-finite {face:?} visual offset")]
    InvalidOffset { tile: String, face: TileFace },
    #[error("tile `{tile}`'s {face:?} animation needs at least one frame and a speed above zero")]
    InvalidAnimation { tile: String, face: TileFace },
    #[error("tile `{0}` has a glow that is not a finite number of at least zero")]
    InvalidGlow(String),
    #[error("tile `{tile}` has height {height}, which must be above zero and at most one cell")]
    InvalidHeight { tile: String, height: f32 },
}

const fn yes() -> bool { true }
const fn one_weight() -> u32 { 1 }
#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_one(weight: &u32) -> bool { *weight == 1 }
const fn full_height() -> f32 { 1.0 }
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_full_height(value: &f32) -> bool { value.to_bits() == full_height().to_bits() }
#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_true(value: &bool) -> bool { *value }
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero(value: &f32) -> bool { *value == 0.0 }
#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_zero_vec(value: &[f32; 2]) -> bool { value[0] == 0.0 && value[1] == 0.0 }

#[cfg(test)]
mod tests;