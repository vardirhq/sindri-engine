//! The sprite shaders a project ships.

use crate::ExportError;

/// Refuses a sprite shader that does not compile, naming its line.
///
/// Checked here because a browser build cannot say which line broke: the
/// export runs where the compiler can, so a shipped project only carries
/// shaders that are known to build.
pub(crate) fn validate(id: &str, bytes: &[u8]) -> Result<(), ExportError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|error| ExportError::Project(format!("{id}: {error}")))?;
    sindri_scene::SpriteShaderBindings::default()
        .bind(id, source)
        .map_err(|error| ExportError::Project(error.to_string()))
}
