use std::sync::Arc;

use glam::Mat4;

use crate::{
    FrameCamera, FrameTarget, ModelCacheStats, ModelRenderError, RenderModel, model::ModelDraw,
};

use super::TexturedCubeRenderer;

impl TexturedCubeRenderer {
    /// Persistent imported geometry and texture upload statistics.
    pub const fn model_cache_stats(&self) -> ModelCacheStats {
        self.models.stats()
    }

    pub(crate) fn encode_shadow_model(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        asset: &Arc<RenderModel>,
        world: Mat4,
    ) -> Result<(), ModelRenderError> {
        self.models.encode_shadow(
            device,
            queue,
            encoder,
            asset,
            crate::model::ModelShadowDraw {
                world,
                projection: self.shadow_view_projection,
                target: self.shadow_map.view(),
                material: self.material,
                sidedness: self.sidedness,
            },
        )
    }

    /// Draws a shared model using entity and camera transforms.
    pub fn encode_model(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: FrameTarget<'_>,
        asset: &Arc<RenderModel>,
        matrices: (Mat4, FrameCamera),
    ) -> Result<(), ModelRenderError> {
        self.encode_model_occluded(
            device,
            queue,
            encoder,
            target,
            asset,
            (matrices.0, matrices.1, None),
        )
    }

    /// Draws normally, or draws only fragments hidden behind existing depth.
    pub fn encode_model_occluded(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: FrameTarget<'_>,
        asset: &Arc<RenderModel>,
        matrices: (Mat4, FrameCamera, Option<[f32; 4]>),
    ) -> Result<(), ModelRenderError> {
        self.models.encode(
            device,
            queue,
            encoder,
            target,
            asset,
            &ModelDraw {
                world: matrices.0,
                camera: matrices.1,
                lighting: self.lighting,
                shadow_map: &self.shadow_map,
                light_view_projection: self.shadow_view_projection,
                shadows: self.shadow_settings,
                fog: self.fog,
                material: self.material,
                sidedness: self.sidedness,
                occluded_color: matrices.2,
            },
        )
    }
}
