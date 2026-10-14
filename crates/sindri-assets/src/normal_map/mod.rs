//! Making a normal map from a picture.
//!
//! A sprite or tilemap that names a `normal_map` is lit by how its surface
//! faces each light (`docs/2d-lights.md`). This derives that map from the
//! sprite itself, or from a height map painted for it, so a map does not have
//! to be drawn by hand. It works on decoded RGBA8 pixels and returns RGBA8
//! pixels: no GPU, no window and no editor, so the editor's Normal Map Studio,
//! a test and any future command-line tool make the same bytes from the same
//! input. `docs/normal-map-studio.md` is the contract.

mod document;
mod generate;
mod remake;
mod settings;

#[cfg(test)]
mod tests;

pub use document::{
    NORMAL_MAP_EXTENSION, NORMAL_MAP_FORMAT_VERSION, NormalMapDocument, NormalMapDocumentError,
    normal_map_paths,
};
pub use generate::{
    FLAT_NORMAL, NormalMapError, Pixels, cells_of_sheet, encode_png, generate, height_field,
};
pub use remake::{Picture, Remade, RemakeError, remake};
pub use settings::{EdgeMode, GreenAxis, HeightSource, NormalMapPreset, NormalMapSettings};
