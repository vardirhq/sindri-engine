//! Authored model collision references and host asset loading.

use std::collections::BTreeMap;

use sindri_core::{
    AssetId, ComponentSchemaRegistry, SceneComponent, SceneDocument, SceneEntity, World,
};
use sindri_decay::PrefabSources;
use sindri_scene::{ModelCollider3dComponent, ModelConvexCollider3dComponent};

/// Collision geometry requested for a logical asset. A project may need both
/// representations on different entities, but loads the original bytes once.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ModelCollisionUse {
    pub triangles: bool,
    pub convex: bool,
}
/// Deduplicated logical assets and all collision representations they need.
pub type ModelCollisionReferences = BTreeMap<AssetId, ModelCollisionUse>;

/// Merges uses without discarding either representation of a shared asset.
pub fn merge_references(target: &mut ModelCollisionReferences, source: ModelCollisionReferences) {
    for (id, usage) in source {
        let entry = target.entry(id).or_default();
        entry.triangles |= usage.triangles;
        entry.convex |= usage.convex;
    }
}

/// Collision assets named by all scenes and spawnable prefabs, including
/// inactive entities and instance overrides. Rendering is independent.
///
/// # Errors
/// Invalid prefab expansion or component data.
pub fn collision_references(
    scenes: &[(String, SceneDocument)],
    prefabs: &PrefabSources,
    components: &ComponentSchemaRegistry,
) -> Result<ModelCollisionReferences, String> {
    let mut wanted = ModelCollisionReferences::new();
    for (_, scene) in scenes {
        collect(
            &scene
                .expanded(prefabs)
                .map_err(|error| error.to_string())?
                .entities,
            components,
            &mut wanted,
        )?;
    }
    for prefab in prefabs.ids().filter_map(|id| prefabs.get(id)) {
        collect(
            &prefab
                .expanded(prefabs)
                .map_err(|error| error.to_string())?
                .entities,
            components,
            &mut wanted,
        )?;
    }
    Ok(wanted)
}

/// Includes unsaved editor changes and inactive entities in a loaded world.
///
/// # Errors
/// Invalid model-collider component data.
pub fn collision_world_references(
    world: &World,
    components: &ComponentSchemaRegistry,
) -> Result<ModelCollisionReferences, String> {
    let mut wanted = ModelCollisionReferences::new();
    for (_, entity) in world.entities() {
        if let Some(payload) = entity.components.get(ModelCollider3dComponent::TYPE_NAME) {
            let value = components
                .decode::<ModelCollider3dComponent>(payload)
                .map_err(|error| error.to_string())?;
            wanted.entry(value.asset).or_default().triangles = true;
        }
        if let Some(payload) = entity
            .components
            .get(ModelConvexCollider3dComponent::TYPE_NAME)
        {
            let value = components
                .decode::<ModelConvexCollider3dComponent>(payload)
                .map_err(|error| error.to_string())?;
            wanted.entry(value.0.asset).or_default().convex = true;
        }
    }
    Ok(wanted)
}

fn collect(
    entities: &[SceneEntity],
    components: &ComponentSchemaRegistry,
    wanted: &mut ModelCollisionReferences,
) -> Result<(), String> {
    for entity in entities {
        if let Some(payload) = entity.components.get(ModelCollider3dComponent::TYPE_NAME) {
            let value = components
                .decode::<ModelCollider3dComponent>(payload)
                .map_err(|error| error.to_string())?;
            wanted.entry(value.asset).or_default().triangles = true;
        }
        if let Some(payload) = entity
            .components
            .get(ModelConvexCollider3dComponent::TYPE_NAME)
        {
            let value = components
                .decode::<ModelConvexCollider3dComponent>(payload)
                .map_err(|error| error.to_string())?;
            wanted.entry(value.0.asset).or_default().convex = true;
        }
    }
    Ok(())
}

/// Loads authored collision assets through the native worker queue, waiting
/// before headless gameplay starts. It never prepares render-only geometry.
///
/// # Errors
/// Queue creation, missing/malformed assets or invalid collision geometry,
/// with the logical asset ID for asset failures.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_collision(
    root: &std::path::Path,
    wanted: &ModelCollisionReferences,
    session: &mut crate::Session,
) -> Result<(), String> {
    use sindri_assets::{
        AssetLoadOutcome, AssetLoadQueueConfig, AssetLoader, FileSystemAssetSource,
        ModelAssetDecoder,
    };

    if wanted.is_empty() {
        return Ok(());
    }
    let mut loader = AssetLoader::new(
        FileSystemAssetSource::new(root),
        AssetLoadQueueConfig::unbounded(2),
        ModelAssetDecoder,
    )
    .map_err(|error| error.to_string())?;
    for id in wanted.keys() {
        loader
            .request(id.clone())
            .map_err(|error| format!("{id}: {error}"))?;
    }
    loop {
        for outcome in loader.poll() {
            match outcome {
                AssetLoadOutcome::Ready(id) => {
                    let asset = loader
                        .get(&id)
                        .ok_or_else(|| format!("{id}: loaded model is unavailable"))?;
                    for warning in &asset.warnings {
                        log::warn!("{id}: {warning}");
                    }
                    session
                        .bind_model_collisions(id.clone(), asset, wanted[&id])
                        .map_err(|error| format!("{id}: {error}"))?;
                }
                AssetLoadOutcome::Failed(error) => {
                    return Err(format!("{}: {}", error.id(), error.message()));
                }
            }
        }
        if loader.outstanding() == 0 {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
