use std::{collections::BTreeSet, sync::Arc};

use sindri_core::{AssetId, SceneComponent, World};
use sindri_render::{
    ExtractedFrame, FrameCamera, FrameCommand, FramePass, RenderLayer, RenderModel, RenderStage,
};

use super::{SceneExtractError, SceneExtractor, camera::ResolvedCameras, transform_matrix};
use crate::ModelComponent;

/// Every model named by the world, including currently inactive entities.
///
/// Hosts and the exporter load once per logical ID. Component validation
/// reports malformed references before this discovery is used.
pub fn referenced_models(world: &World) -> BTreeSet<String> {
    world
        .entities()
        .filter_map(|(_, data)| data.components.get(ModelComponent::TYPE_NAME))
        .filter_map(|payload| payload.get("asset"))
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect()
}

impl SceneExtractor {
    /// Binds a resource once; every entity with this asset shares its geometry.
    pub fn bind_model(&mut self, id: AssetId, model: Arc<RenderModel>) -> Option<Arc<RenderModel>> {
        self.models.insert(id, model)
    }

    /// Whether `id` is bound.
    #[must_use]
    pub fn has_model(&self, id: &AssetId) -> bool {
        self.models.contains_key(id)
    }

    /// Releases the binding. Existing frame packets keep their resource alive.
    pub fn unbind_model(&mut self, id: &AssetId) -> Option<Arc<RenderModel>> {
        self.models.remove(id)
    }

    pub(super) fn push_models(
        &self,
        world: &World,
        cameras: &ResolvedCameras,
        frame: &mut ExtractedFrame,
    ) -> Result<(), SceneExtractError> {
        for (entity, component) in self.components.query::<ModelComponent>(world)? {
            let Some(asset) = self.models.get(&component.asset) else {
                // An editor draws the rest of the scene while a model loads,
                // or when its file will not; a game binds every model first.
                let error = SceneExtractError::MissingModel(component.asset.clone());
                if self.tolerant() {
                    self.record(entity, ModelComponent::TYPE_NAME, &error);
                    continue;
                }
                return Err(error);
            };
            let camera = cameras.world.ok_or(SceneExtractError::MissingWorldCamera)?;
            let model = transform_matrix(world.world_transform(entity).unwrap_or_default());
            if !model.is_finite() || model.determinant().abs() < 1e-12 {
                let error = sindri_render::ModelRenderError::InvalidTransform;
                if self.tolerant() {
                    self.record(entity, ModelComponent::TYPE_NAME, &error);
                    continue;
                }
                return Err(error.into());
            }
            frame.push(FramePass::new(
                RenderStage::Opaque3d,
                RenderLayer(component.layer),
                FrameCamera {
                    view_projection: camera.view_projection,
                    position: camera.view.inverse().transform_point3(glam::Vec3::ZERO),
                },
                FrameCommand::Model {
                    model,
                    asset: Arc::clone(asset),
                },
            ));
        }
        Ok(())
    }
}
