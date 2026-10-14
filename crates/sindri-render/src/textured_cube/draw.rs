use super::uniform::cube_uniform;
use crate::{DepthTarget, FogSettings, MeshBuffers, MeshSurface, ShadowSettings, WorldLighting};
use glam::{Mat4, Vec3};

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn encode_mesh_buffers(
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    target: (&wgpu::TextureView, &DepthTarget),
    model: Mat4,
    model_view_projection: Mat4,
    lighting: WorldLighting,
    light_view_projection: Mat4,
    shadows: ShadowSettings,
    ambient_occlusion_strength: f32,
    fog: FogSettings,
    camera_position: Vec3,
    surface: MeshSurface,
    material: Option<crate::MaterialFactors>,
    state: (&wgpu::Buffer, &wgpu::RenderPipeline, &wgpu::BindGroup),
    mesh: &MeshBuffers,
    label: &str,
    counters: &mut crate::MeshDrawStats,
) {
    let (uniform, pipeline, bind_group) = state;
    queue.write_buffer(
        uniform,
        0,
        bytemuck::bytes_of(&cube_uniform(
            model,
            model_view_projection,
            lighting,
            light_view_projection,
            shadows,
            ambient_occlusion_strength,
            fog,
            camera_position,
            surface,
            material,
        )),
    );
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target.0,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: target.1.view(),
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    mesh.draw(&mut pass);
    counters.record_draw(std::mem::size_of::<super::uniform::CubeUniform>());
}
