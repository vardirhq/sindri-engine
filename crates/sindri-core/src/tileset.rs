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

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bottom => "bottom",
            Self::North => "north",
            Self::West => "west",
            Self::East => "east",
            Self::South => "south",
            Self::Top => "top",
        }
    }
}

impl Serialize for TileFace {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for TileFace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "bottom" => Ok(Self::Bottom),
            "north" => Ok(Self::North),
            "west" => Ok(Self::West),
            "east" => Ok(Self::East),
            "south" => Ok(Self::South),
            "top" => Ok(Self::Top),
            _ => Err(serde::de::Error::custom(format!("unknown tile face `{value}`"))),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TileFaceVisual {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation: Option<FaceAnimation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tint: Option<[f32; 4]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FaceAnimation {
    pub frames: Vec<String>,
    #[serde(default = "default_fps")]
    pub fps: f32,
}

const fn default_fps() -> f32 {
    8.0
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TileFaces {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<TileFaceVisual>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom: Option<TileFaceVisual>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub north: Option<TileFaceVisual>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub south: Option<TileFaceVisual>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub east: Option<TileFaceVisual>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub west: Option<TileFaceVisual>,
}

impl TileFaces {
    #[must_use]
    pub fn get(&self, face: TileFace) -> Option<&TileFaceVisual> {
        match face {
            TileFace::Top => self.top.as_ref(),
            TileFace::Bottom => self.bottom.as_ref(),
            TileFace::North => self.north.as_ref(),
            TileFace::South => self.south.as_ref(),
            TileFace::East => self.east.as_ref(),
            TileFace::West => self.west.as_ref(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TileDefinition {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default)]
    pub solid: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faces: Option<TileFaces>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<Vec<TileBox>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TileSetDocument {
    pub format_version: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default)]
    pub tiles: BTreeMap<String, TileDefinition>,
}

impl Default for TileSetDocument {
    fn default() -> Self {
        Self { format_version: TILESET_FORMAT_VERSION, name: String::new(), tiles: BTreeMap::new() }
    }
}

impl TileSetDocument {
    pub fn from_json(json: &str) -> Result<Self, TileSetError> {
        let document: Self = serde_json::from_str(json)?;
        document.validate()?;
        Ok(document)
    }

    pub fn to_canonical_json(&self) -> Result<String, TileSetError> {
        self.validate()?;
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        Ok(json)
    }

    pub fn validate(&self) -> Result<(), TileSetError> {
        if self.format_version != TILESET_FORMAT_VERSION {
            return Err(TileSetError::UnsupportedVersion { found: self.format_version, supported: TILESET_FORMAT_VERSION });
        }
        for (id, tile) in &self.tiles {
            if id.trim().is_empty() {
                return Err(TileSetError::EmptyId);
            }
            if let Some(shape) = &tile.shape {
                for (index, tile_box) in shape.iter().enumerate() {
                    tile_box.validate().map_err(|message| TileSetError::InvalidShape {
                        tile: id.clone(),
                        index,
                        message,
                    })?;
                }
            }
            if let Some(faces) = &tile.faces {
                for face in TileFace::ALL {
                    if let Some(visual) = faces.get(face) {
                        validate_visual(id, face, visual)?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum TileSetError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("tileset format {found} is unsupported; this runtime supports {supported}")]
    UnsupportedVersion { found: u32, supported: u32 },
    #[error("a tile id may not be empty")]
    EmptyId,
    #[error("tile `{tile}` shape box {index} is invalid: {message}")]
    InvalidShape { tile: String, index: usize, message: String },
    #[error("tile `{tile}` face `{face}` is invalid: {message}")]
    InvalidFace { tile: String, face: String, message: String },
}
