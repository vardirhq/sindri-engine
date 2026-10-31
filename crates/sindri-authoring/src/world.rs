//! The authored world a scene file describes.

use sindri_core::{SceneDocument, UnknownComponentPolicy, World};
use sindri_scene::SceneExtractor;

use crate::SceneFile;

/// Builds the world an editor works on from a scene file.
///
/// Each instance is made from its prefab, and the components checked are what
/// the instances expanded to: a prefab's components are as much the scene's as
/// the ones written in it.
///
/// # Errors
/// A document that will not expand or a component that does not validate.
pub fn load_world(extractor: &SceneExtractor, file: &SceneFile) -> Result<World, String> {
    let expanded = file
        .document()
        .expanded(file.prefabs())
        .map_err(|error| error.to_string())?;
    extractor
        .validate(&expanded, UnknownComponentPolicy::Preserve)
        .map_err(|error| error.to_string())?;
    Ok(World::from_scene_with(file.document(), file.prefabs())
        .map_err(|error| error.to_string())?
        .world)
}

/// [`load_world`] for a document with no file behind it, and so no prefabs.
///
/// # Errors
/// As [`load_world`].
pub fn load_document(
    extractor: &SceneExtractor,
    document: &SceneDocument,
) -> Result<World, String> {
    load_world(extractor, &SceneFile::detached(document.clone()))
}
