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
        self.models.encode(
            device,
            queue,
            encoder,
            target,
            asset,
            ModelDraw {
                world: matrices.0,
                camera: matrices.1,
                lighting: self.lighting,
            },
        )
    }
}
