//! An entity and everything under it, as a prefab.

use sindri_core::{
    ComponentSchemaRegistry, EntityId, PrefabDocument, PrefabLibrary, SceneEntityId, World,
};

/// The prefab `entity` and its descendants would make.
///
/// Written the way a save writes them, so an instance inside the subtree stays
/// an instance and the new prefab nests it. The root loses its parent and the
/// place it stands in the scene — that is the instance's — but keeps its
/// depth, which a 2D prefab is drawn by. Editor state stays behind. Registered
/// runtime-local entity fields use the same stable-reference remapping as scenes.
///
/// # Errors
/// A subtree that cannot be written, as a save would say.
pub fn subtree_prefab(
    world: &World,
    entity: EntityId,
    prefabs: &dyn PrefabLibrary,
    components: &ComponentSchemaRegistry,
) -> Result<PrefabDocument, String> {
    let root_id = world
        .get(entity)
        .and_then(|data| data.source_id.clone())
        .ok_or("it has no stable ID")?;
    let scene = world
        .to_scene_with_references(prefabs, components)
        .map_err(|error| error.to_string())?;
    let within: Vec<SceneEntityId> = world
        .capture_subtree(entity)
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter_map(|(_, data)| data.source_id)
        .collect();
    let mut entities: Vec<_> = scene
        .entities
        .into_iter()
        .filter(|written| within.contains(&written.id))
        .collect();
    for written in &mut entities {
        written.editor.clear();
        if written.id == root_id {
            written.parent = None;
            if let Some(transform) = &mut written.transform_3d {
                transform.position[0] = 0.0;
                transform.position[1] = 0.0;
            }
        }
    }
    let document = PrefabDocument {
        entities,
        ..PrefabDocument::default()
    };
    document.validate().map_err(|error| error.to_string())?;
    Ok(document)
}
