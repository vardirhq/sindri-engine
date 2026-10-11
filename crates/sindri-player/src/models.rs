//! Model rendering and collision are installed after both asset sources finish.

use std::collections::BTreeSet;

use sindri_assets::ModelAsset;
use sindri_core::{AssetId, ComponentSchemaRegistry, SceneComponent, SceneDocument, SceneEntity};
use sindri_decay::PrefabSources;
use sindri_runtime::Session;
use sindri_scene::{ModelCollider3dComponent, SceneExtractor};

use crate::PlayerError;

/// Include later scenes, inactive entities and spawnable/nested prefabs.
/// Expand first so instance overrides use the same references as gameplay.
pub(super) fn references(
    scenes: &[(String, SceneDocument)],
    prefabs: &PrefabSources,
    components: &ComponentSchemaRegistry,
) -> Result<BTreeSet<AssetId>, PlayerError> {
    let mut wanted = BTreeSet::new();
    for (_, scene) in scenes {
        collect(&scene.expanded(prefabs)?.entities, components, &mut wanted)?;
    }
    for prefab in prefabs.ids().filter_map(|id| prefabs.get(id)) {
        collect(&prefab.expanded(prefabs)?.entities, components, &mut wanted)?;
    }
    Ok(wanted)
}

fn collect(
    entities: &[SceneEntity],
    components: &ComponentSchemaRegistry,
    wanted: &mut BTreeSet<AssetId>,
) -> Result<(), PlayerError> {
    for entity in entities {
        if let Some(payload) = entity.components.get(ModelCollider3dComponent::TYPE_NAME) {
            wanted.insert(
                components
                    .decode::<ModelCollider3dComponent>(payload)?
                    .asset,
            );
        }
    }
    Ok(())
}

/// Prepare collision only when authored: a render-only GLB need not be a
/// valid physics surface. The host starts no gameplay until this succeeds.
pub(super) fn install(
    models: Vec<(AssetId, ModelAsset)>,
    collision: &BTreeSet<AssetId>,
    scene: &mut SceneExtractor,
    session: &mut Session,
) -> Result<(), PlayerError> {
    if let Some(missing) = collision
        .iter()
        .find(|id| !models.iter().any(|(loaded, _)| loaded == *id))
    {
        return Err(PlayerError::MissingModelCollision(missing.clone()));
    }
    for (id, asset) in models {
        if collision.contains(&id) {
            session
                .bind_model_collision(id.clone(), &asset)
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
