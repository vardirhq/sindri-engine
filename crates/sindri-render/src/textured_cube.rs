use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::{
    CachedMeshId, DepthTarget, FogSettings, MeshBuffers, MeshSurface, RegistryIdentity,
    ShadowSettings, TextureId, TextureRegistry, TexturedMeshCacheStats, TexturedVertex,
    WorldLighting,
    shadow::{ShadowMap, create_shadow_pipeline},
    textured_mesh_cache::TexturedMeshCache,
};

const SHADER: &str = concat!(
    include_str!("world_effects.wgsl"),
    include_str!("material.wgsl"),
    include_str!("textured_cube.wgsl")
);

mod draw;
mod geometry;
mod material;
mod model;
mod pipeline;
mod retained;
mod shadow_pass;
mod uniform;

use draw::encode_mesh_buffers;
use uniform::cube_uniform;

use geometry::{INDICES, VERTICES};
use pipeline::{create_bind_group_layout, create_pipeline, orientation};

#[derive(Debug)]
pub struct TexturedCubeRenderer {
    models: crate::model::ModelRenderer,
    draw_stats: crate::MeshDrawStats,
    pipeline: [wgpu::RenderPipeline; 4],
    bind_group_layout: wgpu::BindGroupLayout,
    batches: Vec<MeshBatch>,
    next: usize,
    mesh: MeshBuffers,
    cached_meshes: TexturedMeshCache,
    retained_meshes: crate::retained_mesh::RetainedMeshes,
    material: Option<crate::MaterialFactors>,
    sidedness: crate::MaterialSidedness,
    lighting: WorldLighting,
    shadow_settings: ShadowSettings,
    shadow_map: ShadowMap,
    shadow_bind_group_layout: wgpu::BindGroupLayout,
    shadow_pipeline: [wgpu::RenderPipeline; 4],
    shadow_view_projection: Mat4,
    ambient_occlusion_strength: f32,
    fog: FogSettings,
    camera_position: Vec3,
}

/// GPU state owned by one textured-mesh draw in a submission.
///
/// Queue writes land before the submitted command buffer runs, so sharing a
/// uniform across draws would make every mesh use the final draw's transform.
#[derive(Debug)]
struct MeshBatch {
    uniform: wgpu::Buffer,
    shadow_uniform: wgpu::Buffer,
    shadow_bind_group: wgpu::BindGroup,
    bind_groups: std::collections::HashMap<TextureId, wgpu::BindGroup>,
    /// The registry `bind_groups` were made from; see the sprite batch.
    registry: RegistryIdentity,
}

mod context;
pub(crate) use context::CachedMeshRequest;
pub use context::DrawContext;

impl TexturedCubeRenderer {
    /// Textures come from the frame's [`TextureRegistry`] rather than being
    /// baked in, so one renderer draws every mesh in a scene.
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let bind_group_layout = create_bind_group_layout(device);
        let pipeline = [
            (wgpu::FrontFace::Ccw, false),
            (wgpu::FrontFace::Cw, false),
            (wgpu::FrontFace::Ccw, true),
            (wgpu::FrontFace::Cw, true),
        ]
        .map(|(front, double)| {
            create_pipeline(device, target_format, &bind_group_layout, front, double)
        });
        let shadow_map = ShadowMap::new(device, ShadowSettings::default().map_size);
        let (shadow_bind_group_layout, shadow_pipeline) = create_shadow_pipeline(device);
        Self {
            models: crate::model::ModelRenderer::new(device, target_format),
            draw_stats: crate::MeshDrawStats::default(),
            pipeline,
            bind_group_layout,
            batches: Vec::new(),
            next: 0,
            mesh: MeshBuffers::new(device, "Sindri textured cube", &VERTICES, &INDICES),
            cached_meshes: TexturedMeshCache::default(),
            retained_meshes: crate::retained_mesh::RetainedMeshes::default(),
            material: None,
            sidedness: crate::MaterialSidedness::Inherit,
            lighting: WorldLighting::default(),
            shadow_settings: ShadowSettings::default(),
            shadow_map,
            shadow_bind_group_layout,
            shadow_pipeline,
            shadow_view_projection: Mat4::IDENTITY,
            ambient_occlusion_strength: 0.0,
            fog: FogSettings::default(),
            camera_position: Vec3::ZERO,
        }
    }

    /// Sets the world lighting used by subsequent textured 3D draws.
    pub fn set_lighting(&mut self, lighting: WorldLighting) {
        self.lighting = lighting;
    }

    pub fn set_shadows(&mut self, device: &wgpu::Device, settings: ShadowSettings) {
        self.shadow_settings = settings;
        if self.shadow_map.resize(device, settings.map_size) {
            self.models.invalidate_bindings();
            for batch in &mut self.batches {
                batch.bind_groups.clear();
            }
        }
    }

    /// Sets how strongly mesh-authored ambient occlusion darkens world geometry.
    pub fn set_ambient_occlusion(&mut self, strength: f32) {
        self.ambient_occlusion_strength = strength.clamp(0.0, 1.0);
    }

    /// Sets atmosphere applied to subsequent opaque world draws.
    pub fn set_fog(&mut self, fog: FogSettings) {
        self.fog = fog;
    }

    pub fn set_camera_position(&mut self, position: Vec3) {
        self.camera_position = position;
    }

    pub(crate) const fn shadows_enabled(&self) -> bool {
        self.shadow_settings.enabled && self.lighting.directional_intensity > 0.0
    }

    /// Makes the reusable draw slots available for a new GPU submission.
    pub fn begin_submission(&mut self) {
        self.next = 0;
        self.models.begin_submission();
        self.retained_meshes.begin_submission();
    }

    fn reserve(&mut self, device: &wgpu::Device) -> usize {
        let slot = self.next;
        self.next += 1;
        if slot == self.batches.len() {
            let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sindri textured mesh uniform"),
                contents: bytemuck::bytes_of(&cube_uniform(
                    Mat4::IDENTITY,
                    Mat4::IDENTITY,
                    &WorldLighting::default(),
                    Mat4::IDENTITY,
                    ShadowSettings::default(),
                    0.0,
                    FogSettings::default(),
                    Vec3::ZERO,
                    MeshSurface::default(),
                    self.material,
                )),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let shadow_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sindri shadow mesh uniform"),
                contents: bytemuck::cast_slice(&Mat4::IDENTITY.to_cols_array()),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Sindri shadow mesh bind group"),
                layout: &self.shadow_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: shadow_uniform.as_entire_binding(),
                }],
            });
            self.batches.push(MeshBatch {
                uniform,
                shadow_uniform,
                shadow_bind_group,
                bind_groups: std::collections::HashMap::new(),
                registry: RegistryIdentity::default(),
            });
        }
        slot
    }

    /// Returns the bind group for `texture`, creating it on first use.
    fn bind_texture(
        &mut self,
        device: &wgpu::Device,
        registry: &TextureRegistry,
        slot: usize,
        texture: TextureId,
    ) {
        let batch = &mut self.batches[slot];
        if batch.registry != registry.identity() {
            batch.bind_groups.clear();
            batch.registry = registry.identity();
        }
        if batch.bind_groups.contains_key(&texture) {
            return;
        }
        let resolved = registry.get(texture);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sindri textured cube bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: batch.uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(resolved.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(resolved.sampler()),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(self.shadow_map.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(self.shadow_map.sampler()),
                },
            ],
        });
        batch.bind_groups.insert(texture, bind_group);
    }

    /// Draws the cube into a frame something else has already cleared.
    ///
    /// Both attachments load: a renderer draws one thing, and deciding what the
    /// rest of the frame starts as is not its to make. See
    /// [`encode_clear`](crate::encode_clear).
    pub fn encode(
        &mut self,
        context: DrawContext<'_>,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &DepthTarget,
        model_view_projection: Mat4,
    ) {
        let slot = self.reserve(context.device);
        self.bind_texture(context.device, context.textures, slot, context.texture);
        let batch = &self.batches[slot];
        let bind_group = batch
            .bind_groups
            .get(&context.texture)
            .expect("the texture is bound before encoding");
        encode_mesh_buffers(
            context.queue,
            encoder,
            (target, depth),
            Mat4::IDENTITY,
            model_view_projection,
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
                &self.pipeline[orientation(Mat4::IDENTITY, self.sidedness)],
                bind_group,
            ),
            &self.mesh,
            "Sindri textured cube pass",
            &mut self.draw_stats,
        );
    }

    /// Draws a cube with a separate model transform so lighting stays in world space.
    pub fn encode_world(
        &mut self,
        context: DrawContext<'_>,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &DepthTarget,
        model: Mat4,
        view_projection: Mat4,
    ) {
        let slot = self.reserve(context.device);
        self.bind_texture(context.device, context.textures, slot, context.texture);
        let batch = &self.batches[slot];
        let bind_group = batch
            .bind_groups
            .get(&context.texture)
            .expect("the texture is bound before encoding");
        encode_mesh_buffers(
            context.queue,
            encoder,
            (target, depth),
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
                &self.pipeline[orientation(model, self.sidedness)],
                bind_group,
            ),
            &self.mesh,
            "Sindri textured cube pass",
            &mut self.draw_stats,
        );
    }

    /// Draws caller-provided textured triangles through the same opaque 3D
    /// pipeline as cubes.
    ///
    /// This low-level transient path uploads on every draw. Scene surfaces use
    /// retained resources; generated terrain uses [`Self::encode_cached_mesh`].
    #[allow(clippy::too_many_lines)]
    pub fn encode_mesh(
        &mut self,
        context: DrawContext<'_>,
        encoder: &mut wgpu::CommandEncoder,
        target: (&wgpu::TextureView, &DepthTarget),
        model_view_projection: Mat4,
        vertices: &[TexturedVertex],
        indices: &[u16],
    ) {
        if vertices.is_empty() || indices.is_empty() {
            return;
        }
        let mesh = MeshBuffers::new(context.device, "Sindri textured surface", vertices, indices);
        self.draw_stats.transient_geometry_uploads += 1;
        let slot = self.reserve(context.device);
        self.bind_texture(context.device, context.textures, slot, context.texture);
        let batch = &self.batches[slot];
        let bind_group = batch
            .bind_groups
            .get(&context.texture)
            .expect("the texture is bound before encoding");
        encode_mesh_buffers(
            context.queue,
            encoder,
            target,
            Mat4::IDENTITY,
            model_view_projection,
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
                &self.pipeline[orientation(Mat4::IDENTITY, self.sidedness)],
                bind_group,
            ),
            &mesh,
            "Sindri textured surface pass",
            &mut self.draw_stats,
        );
    }

    /// Draws caller-provided textured triangles with world-space lighting.
    #[allow(clippy::too_many_arguments)]
    pub fn encode_mesh_world(
        &mut self,
        context: DrawContext<'_>,
        encoder: &mut wgpu::CommandEncoder,
        target: (&wgpu::TextureView, &DepthTarget),
        model: Mat4,
        view_projection: Mat4,
        vertices: &[TexturedVertex],
        indices: &[u16],
    ) {
        if vertices.is_empty() || indices.is_empty() {
            return;
        }
        let mesh = MeshBuffers::new(context.device, "Sindri textured surface", vertices, indices);
        self.draw_stats.transient_geometry_uploads += 1;
        let slot = self.reserve(context.device);
        self.bind_texture(context.device, context.textures, slot, context.texture);
        let batch = &self.batches[slot];
        let bind_group = batch
            .bind_groups
            .get(&context.texture)
            .expect("the texture is bound before encoding");
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
                &self.pipeline[orientation(model, self.sidedness)],
                bind_group,
            ),
            &mesh,
            "Sindri textured surface pass",
            &mut self.draw_stats,
        );
    }

    /// Draws a persistent textured mesh, replacing its GPU buffers only when a
    /// newer revision supplies geometry.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn encode_cached_mesh(
        &mut self,
        context: DrawContext<'_>,
        encoder: &mut wgpu::CommandEncoder,
        target: (&wgpu::TextureView, &DepthTarget),
        model: Mat4,
        view_projection: Mat4,
        request: CachedMeshRequest<'_>,
    ) {
        let slot = self.reserve(context.device);
        self.bind_texture(context.device, context.textures, slot, context.texture);
        let Some(mesh) = self.cached_meshes.resolve(
            context.device,
            request.id,
            request.revision,
            request.replacement,
        ) else {
            return;
        };
        let batch = &self.batches[slot];
        let bind_group = batch
            .bind_groups
            .get(&context.texture)
            .expect("the texture is bound before encoding");
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
            request.surface,
            self.material,
            (
                &batch.uniform,
                &self.pipeline[orientation(model, self.sidedness)],
                bind_group,
            ),
            mesh,
            "Sindri cached textured mesh pass",
            &mut self.draw_stats,
        );
    }

    /// Releases one persistent mesh, normally from a residency `left` delta.
    pub fn release_cached_mesh(&mut self, cache: CachedMeshId) -> bool {
        self.cached_meshes.release(cache)
    }

    /// Combined retained inline/voxel cache counters and resident entry count.
    pub fn cached_mesh_stats(&self) -> TexturedMeshCacheStats {
        self.cached_meshes.stats().and(self.retained_meshes.stats())
    }

    /// Cumulative physical 3D draw counters, including imported models and shadows.
    /// Slot reuse at submission boundaries does not reset these counters.
    pub fn draw_stats(&self) -> crate::MeshDrawStats {
        self.draw_stats.and(self.models.draw_stats())
    }

    /// High-water mark of textured mesh draws in one submission.
    #[must_use]
    pub fn batch_slots(&self) -> usize {
        self.batches.len()
    }
}
