//! Model rendering and collision are installed after both asset sources finish.

use sindri_assets::ModelAsset;
use sindri_core::{AssetId, ComponentSchemaRegistry, SceneDocument};
use sindri_decay::PrefabSources;
use sindri_runtime::{Session, model_assets::ModelCollisionReferences};
use sindri_scene::SceneExtractor;

use crate::PlayerError;

/// Include later scenes, inactive entities and spawnable/nested prefabs.
/// Expand first so instance overrides use the same references as gameplay.
pub(super) fn references(
    scenes: &[(String, SceneDocument)],
    prefabs: &PrefabSources,
    components: &ComponentSchemaRegistry,
) -> Result<ModelCollisionReferences, PlayerError> {
    sindri_runtime::model_assets::collision_references(scenes, prefabs, components)
        .map_err(PlayerError::ModelCollisionReferences)
}

/// Prepare collision only when authored: a render-only GLB need not be a
/// valid physics surface. The host starts no gameplay until this succeeds.
pub(super) fn install(
    models: Vec<(AssetId, ModelAsset)>,
    collision: &ModelCollisionReferences,
    scene: &mut SceneExtractor,
    session: &mut Session,
) -> Result<(), PlayerError> {
    if let Some(missing) = collision
        .keys()
        .find(|id| !models.iter().any(|(loaded, _)| loaded == *id))
    {
        return Err(PlayerError::MissingModelCollision(missing.clone()));
    }
    for (id, asset) in models {
        if let Some(usage) = collision.get(&id) {
            session
                .bind_model_collisions(id.clone(), &asset, *usage)
                .map_err(|source| PlayerError::ModelCollision {
                    asset: id.clone(),
                    source,
                })?;
        }
        let prepared = sindri::model::prepare(asset)?;
        for warning in prepared.warnings {
            log::warn!("{id}: {warning}");
        }
        scene.bind_model(id, prepared.resource);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
