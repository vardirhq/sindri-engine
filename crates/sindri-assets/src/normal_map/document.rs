//! The settings a map was made with, saved beside it.
//!
//! A normal map is a picture, and a picture cannot say how it was made. When
//! the art it was made from changes, the map is out of date, and making it
//! again by hand means remembering every number. So the settings are written
//! down next to it, naming the picture they were applied to, and the map is
//! made again from the new art with the same settings.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::settings::NormalMapSettings;

/// The current layout of a settings file.
pub const NORMAL_MAP_FORMAT_VERSION: u32 = 1;

/// The extension a settings file carries: `tiles_n.normalmap` made
/// `tiles_n.png`.
pub const NORMAL_MAP_EXTENSION: &str = "normalmap";

/// How a normal map was made.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NormalMapDocument {
    pub format_version: u32,
    /// The picture the map was made from, relative to this file.
    pub source: String,
    /// The height map it was made from, relative to this file, when the
    /// settings read one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height_map: Option<String>,
    #[serde(default)]
    pub settings: NormalMapSettings,
}

/// Why a settings file could not be read.
#[derive(Debug, Error)]
pub enum NormalMapDocumentError {
    #[error("not a normal map settings file: {0}")]
    Json(#[from] serde_json::Error),
    #[error("normal map settings version {0} is newer than this build reads")]
    Version(u32),
}

impl NormalMapDocument {
    /// The settings for `source`, at the current version.
    pub fn new(source: impl Into<String>, settings: NormalMapSettings) -> Self {
        Self {
            format_version: NORMAL_MAP_FORMAT_VERSION,
            source: source.into(),
            height_map: None,
            settings,
        }
    }

    pub fn from_json(json: &str) -> Result<Self, NormalMapDocumentError> {
        let document: Self = serde_json::from_str(json)?;
        if document.format_version > NORMAL_MAP_FORMAT_VERSION {
            return Err(NormalMapDocumentError::Version(document.format_version));
        }
        Ok(document)
    }

    /// Pretty JSON with a trailing newline, so the file diffs like the
    /// project's other documents.
    pub fn to_json(&self) -> String {
        let mut json = serde_json::to_string_pretty(self)
            .expect("a document of strings, numbers and enums always serializes");
        json.push('\n');
        json
    }
}

/// Where the map made from `source` is written, and where its settings are:
/// `textures/floor.png` makes `textures/floor_n.png` and
/// `textures/floor_n.normalmap`.
///
/// `_n` because it is what the platformer's hand-made `tiles_n.png` is
/// already called, and it keeps a map sorted beside the picture it lights.
pub fn normal_map_paths(source: &Path) -> (PathBuf, PathBuf) {
    let stem = source
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    let map = source.with_file_name(format!("{stem}_n.png"));
    let settings = source.with_file_name(format!("{stem}_n.{NORMAL_MAP_EXTENSION}"));
    (map, settings)
}
