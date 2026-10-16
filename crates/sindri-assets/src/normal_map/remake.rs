//! Making a map again from the settings file saved beside it.
//!
//! The settings file is what makes a map nondestructive: it names the picture,
//! the height map if there is one, and the settings, so the map can always be
//! made again from the art as it is now. This is that step, shared by the
//! studio and anything else that wants to check or refresh a saved map.

use std::path::{Path, PathBuf};

use sindri_core::{AssetId, SpriteSheetDocument};
use thiserror::Error;

use super::document::{NormalMapDocument, NormalMapDocumentError};
use super::generate::{NormalMapError, Pixels, cells_of_sheet, encode_png, generate};
use crate::{AssetBytes, AssetDecoder, TextureAssetDecoder};

/// A picture read from disk as RGBA8.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Picture {
    /// Reads and decodes an image file the way the engine decodes a texture.
    pub fn read(path: &Path) -> Result<Self, RemakeError> {
        let bytes = std::fs::read(path).map_err(|error| RemakeError::Read {
            path: path.to_path_buf(),
            error: error.to_string(),
        })?;
        let undecodable = |error: String| RemakeError::Decode {
            path: path.to_path_buf(),
            error,
        };
        let name = path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let id = AssetId::new(&name).map_err(|error| undecodable(error.to_string()))?;
        let asset = TextureAssetDecoder
            .decode(AssetBytes::new(id, bytes))
            .map_err(|error| undecodable(error.to_string()))?;
        Ok(Self {
            width: asset.width(),
            height: asset.height(),
            rgba: asset.rgba8().to_vec(),
        })
    }

    /// The picture as generation's input.
    pub fn pixels(&self) -> Result<Pixels<'_>, NormalMapError> {
        Pixels::new(self.width, self.height, &self.rgba)
    }

    /// The cells of the sheet sliced from this picture, if `path` has one
    /// beside it, as generation keeps them apart.
    #[must_use]
    pub fn cells(&self, path: &Path) -> Vec<[u32; 4]> {
        std::fs::read_to_string(path.with_extension("sheet"))
            .ok()
            .and_then(|json| SpriteSheetDocument::from_json(&json).ok())
            .map(|sheet| cells_of_sheet(&sheet, self.width, self.height))
            .unwrap_or_default()
    }
}

/// A map made again: where it belongs and its PNG bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Remade {
    pub map_path: PathBuf,
    pub png: Vec<u8>,
}

/// Why a map could not be made again.
#[derive(Debug, Error)]
pub enum RemakeError {
    #[error("could not read {path}: {error}", path = .path.display())]
    Read { path: PathBuf, error: String },
    #[error("{path} is not a picture: {error}", path = .path.display())]
    Decode { path: PathBuf, error: String },
    #[error(transparent)]
    Document(#[from] NormalMapDocumentError),
    #[error(transparent)]
    Generate(#[from] NormalMapError),
}

/// Makes the map that the settings file at `settings_path` describes, from
/// the files it names as they are now. The map belongs beside the settings
/// file, with the same name and `.png`.
pub fn remake(settings_path: &Path) -> Result<Remade, RemakeError> {
    let json = std::fs::read_to_string(settings_path).map_err(|error| RemakeError::Read {
        path: settings_path.to_path_buf(),
        error: error.to_string(),
    })?;
    let document = NormalMapDocument::from_json(&json)?;
    let folder = settings_path.parent().unwrap_or_else(|| Path::new(""));
    let source_path = folder.join(&document.source);
    let source = Picture::read(&source_path)?;
    let height_map = document
        .height_map
        .as_ref()
        .map(|name| Picture::read(&folder.join(name)))
        .transpose()?;
    let heights = height_map.as_ref().map(Picture::pixels).transpose()?;
    let map = generate(
        source.pixels()?,
        heights,
        &document.settings,
        &source.cells(&source_path),
    )?;
    Ok(Remade {
        map_path: settings_path.with_extension("png"),
        png: encode_png(source.width, source.height, &map)?,
    })
}
