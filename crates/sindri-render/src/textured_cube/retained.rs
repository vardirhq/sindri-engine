//! Retained surface geometry uses the existing textured color/shadow pipelines.
use super::{DrawContext, TexturedCubeRenderer, draw::encode_mesh_buffers, orientation};
use crate::{DepthTarget, MeshSurface, RenderMesh};
use glam::Mat4;
use std::sync::Arc;

impl TexturedCubeRenderer {
    pub(crate) fn encode_retained_mesh(
        &mut self,
        context: DrawContext<'_>,
        encoder: &mut wgpu::CommandEncoder,
        target: (&wgpu::TextureView, &DepthTarget),
        model: Mat4,
        view_projection: Mat4,
        geometry: &Arc<RenderMesh>,
    ) {
        let slot = self.reserve(context.device);
        self.bind_texture(context.device, context.textures, slot, context.texture);
        let Some(mesh) = self.retained_meshes.resolve(context.device, geometry) else {
            return;
        };
        let batch = &self.batches[slot];
        let bind_group = batch
            .bind_groups
            .get(&context.texture)
            .expect("texture bound before drawing");
        encode_mesh_buffers(
            context.queue,
            encoder,
            target,
            model,
            view_projection * model,
            &self.lighting,
            self.shadow_view_projection,
            self.shadow_settings,
            self.ambient_occlusion_strength,
            self.fog,
            self.camera_position,
            MeshSurface::default(),
            self.material,
            (
                &batch.uniform,
                &self.pipeline[orientation(model)],
                bind_group,
            ),
            mesh,
            "Sindri retained textured mesh pass",
            &mut self.draw_stats,
        );
    }

    pub(crate) fn encode_shadow_retained_mesh(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        model: Mat4,
        geometry: &Arc<RenderMesh>,
    ) {
        let slot = self.reserve(device);
        let Some(mesh) = self.retained_meshes.resolve(device, geometry) else {
            return;
        };
        super::shadow_pass::encode_shadow_mesh(
            queue,
            encoder,
            self.shadow_map.view(),
            self.shadow_view_projection * model,
            &self.batches[slot].shadow_uniform,
            &self.batches[slot].shadow_bind_group,
            &self.shadow_pipeline[orientation(model)],
            mesh,
            &mut self.draw_stats,
        );
    }
}
