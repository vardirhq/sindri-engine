//! The prefabs an authored scene is made from, as files.
//!
//! What an editor does with a prefab — placing one, bringing an instance up to
//! date, turning a selection into one — is the editor's. What lives here is
//! what opening and saving a scene needs: reading the prefabs it places,
//! standing in for one that cannot be read, and treating a prefab opened on
//! its own as a scene document.

pub mod document;
mod library;
pub mod missing;

pub use library::ScenePrefabs;

use std::path::Path;

use sindri_core::PrefabDocument;

/// Reads a prefab from disk, or says why it cannot be used.
///
/// Validated here rather than at instantiation, so a prefab with two roots is
/// refused while it is still an asset somebody chose and not a half-built
/// subtree in their scene.
pub fn load(path: &Path) -> Result<PrefabDocument, String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("this file");
    let json = std::fs::read_to_string(path)
        .map_err(|error| format!("{name} could not be read: {error}"))?;
    let document = PrefabDocument::from_json(&json)
        .map_err(|error| format!("{name} is not valid: {error}"))?;
    document
        .validate()
        .map_err(|error| format!("{name} cannot be instantiated: {error}"))?;
    Ok(document)
}
