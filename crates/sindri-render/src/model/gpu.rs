mod shadow;
pub(crate) use shadow::ModelShadowDraw;

use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use super::{
    ModelCacheStats, ModelIndexData, ModelRenderError, ModelSurface, RenderModel, pipeline,
    validate,
};
use crate::{
    FogSettings, FrameCamera, FrameTarget, ShadowSettings, Texture2D, WorldLighting,
    shadow::ShadowMap,
};

#[derive(Clone, Copy)]
pub(crate) struct ModelDraw<'a> {
    pub world: Mat4,
    pub camera: FrameCamera,
    pub lighting: WorldLighting,
    pub shadow_map: &'a ShadowMap,
    pub light_view_projection: Mat4,
    pub shadows: ShadowSettings,
    pub fog: FogSettings,
    pub material: Option<crate::MaterialFactors>,
    pub sidedness: crate::MaterialSidedness,
    pub occluded_color: Option<[f32; 4]>,
}

#[derive(Debug)]
pub(crate) struct ModelRenderer {
    shadow: shadow::ShadowRenderer,
    pipeline: wgpu::RenderPipeline,
    occluded: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    cache: HashMap<usize, CachedModel>,
    slots: Vec<Slot>,
    next: usize,
    stats: ModelCacheStats,
    draw_stats: crate::MeshDrawStats,
}

#[derive(Debug)]
struct CachedModel {
    owner: Weak<RenderModel>,
    meshes: Vec<Vec<Geometry>>,
    textures: Vec<Texture2D>,
    samplers: Vec<wgpu::Sampler>,
}

#[derive(Debug)]
struct Geometry {
    vertex: wgpu::Buffer,
    index: wgpu::Buffer,
    count: u32,
    format: wgpu::IndexFormat,
}

#[derive(Debug)]
struct Slot {
    uniform: wgpu::Buffer,
    binding: Option<((usize, usize), wgpu::BindGroup)>,
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Uniform {
    view_projection: [[f32; 4]; 4],
    world: [[f32; 4]; 4],
    normal: [[f32; 4]; 4],
    base_color: [f32; 4],
    surface: [f32; 4],
    ambient: [f32; 4],
    direction: [f32; 4],
    light: [f32; 4],
    eye: [f32; 4],
    light_view_projection: [[f32; 4]; 4],
    shadow: [f32; 4],
    fog_color: [f32; 4],
    fog_params: [f32; 4],
    emissive: [f32; 4],
    local_lights: [crate::lighting::LocalLightUniform; crate::MAX_LOCAL_LIGHTS],
    occluded_color: [f32; 4],
}

impl ModelRenderer {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = pipeline::layout(device);
        let pipeline = pipeline::create(device, format, &layout, false);
        let occluded = pipeline::create(device, format, &layout, true);
        Self {
            shadow: shadow::ShadowRenderer::new(device),
            pipeline,
            occluded,
            layout,
            cache: HashMap::new(),
            slots: Vec::new(),
            next: 0,
            stats: ModelCacheStats::default(),
            draw_stats: crate::MeshDrawStats::default(),
        }
    }

    pub(crate) fn invalidate_bindings(&mut self) {
        for slot in &mut self.slots {
            slot.binding = None;
        }
    }

    pub(crate) fn begin_submission(&mut self) {
        self.next = 0;
        self.cache.retain(|_, entry| entry.owner.strong_count() > 0);
        for slot in &mut self.slots {
            if slot
                .binding
                .as_ref()
                .is_some_and(|((key, _), _)| !self.cache.contains_key(key))
            {
                slot.binding = None;
            }
        }
        self.stats.resident_models = self.cache.len();
        self.shadow.begin_submission(&self.cache);
    }

    pub(crate) const fn stats(&self) -> ModelCacheStats {
        self.stats
    }

    pub(crate) const fn draw_stats(&self) -> crate::MeshDrawStats {
        self.draw_stats
    }

    fn ensure_cached(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        asset: &Arc<RenderModel>,
    ) -> Result<usize, ModelRenderError> {
        let key = Arc::as_ptr(asset) as usize;
        if let std::collections::hash_map::Entry::Vacant(entry) = self.cache.entry(key) {
            entry.insert(upload(device, queue, asset)?);
            self.stats.uploads += 1;
            self.stats.resident_models = self.cache.len();
        }
        Ok(key)
    }

    pub(crate) fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: FrameTarget<'_>,
        asset: &Arc<RenderModel>,
        draw: &ModelDraw<'_>,
    ) -> Result<(), ModelRenderError> {
        validate::transform(draw.world)?;
        if draw.occluded_color.is_some_and(|color| {
            color
                .into_iter()
                .any(|c| !c.is_finite() || !(0.0..=1.0).contains(&c))
        }) {
            return Err(ModelRenderError::InvalidData(
                "silhouette RGBA must be finite and in 0..=1".into(),
            ));
        }
        let key = self.ensure_cached(device, queue, asset)?;
        let cached = &self.cache[&key];
        let fallback = ModelSurface::default();
        for instance in asset.instances() {
            let world = draw.world * instance.transform;
            validate::transform(world)?;
            for (index, primitive) in asset.data().meshes[instance.mesh].iter().enumerate() {
                let surface = primitive
                    .material
                    .map_or(&fallback, |i| &asset.data().surfaces[i]);
                let texture = surface.texture.unwrap_or(cached.textures.len() - 1);
                if self.next == self.slots.len() {
                    self.slots.push(Slot {
                        uniform: device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("Imported model draw uniform"),
                            size: std::mem::size_of::<Uniform>() as u64,
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        }),
                        binding: None,
                    });
                }
                let slot = &mut self.slots[self.next];
                self.next += 1;
                queue.write_buffer(
                    &slot.uniform,
                    0,
                    bytemuck::bytes_of(&uniform(draw, world, surface)),
                );
                if slot
                    .binding
                    .as_ref()
                    .is_none_or(|(id, _)| *id != (key, texture))
                {
                    slot.binding = Some((
                        (key, texture),
                        bind(
                            device,
                            &self.layout,
                            &slot.uniform,
                            &cached.textures[texture],
                            &cached.samplers[texture],
                            draw.shadow_map,
                        ),
                    ));
                }
                let geometry = &cached.meshes[instance.mesh][index];
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Imported model"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target.color,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: target.depth.view(),
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    ..Default::default()
                });
                pass.set_pipeline(if draw.occluded_color.is_some() {
                    &self.occluded
                } else {
                    &self.pipeline
                });
                if let Some((_, group)) = &slot.binding {
                    pass.set_bind_group(0, group, &[]);
                }
                pass.set_vertex_buffer(0, geometry.vertex.slice(..));
                pass.set_index_buffer(geometry.index.slice(..), geometry.format);
                pass.draw_indexed(0..geometry.count, 0, 0..1);
                self.stats.draws += 1;
                self.draw_stats.record_draw(std::mem::size_of::<Uniform>());
            }
        }
        Ok(())
    }
}

fn uniform(draw: &ModelDraw<'_>, world: Mat4, surface: &ModelSurface) -> Uniform {
    let factors = draw.material.unwrap_or(crate::MaterialFactors {
        base_color: surface.base_color,
        metallic: surface.metallic,
        roughness: surface.roughness,
        emissive: surface.emissive,
    });
    let lighting = &draw.lighting;
    let ambient = Vec3::from_array(lighting.ambient_color) * lighting.ambient_intensity;
    let light = Vec3::from_array(lighting.directional_color) * lighting.directional_intensity;
    let direction = -Vec3::from_array(lighting.directional_direction);
    Uniform {
        local_lights: lighting.local_uniforms(),
        occluded_color: draw.occluded_color.unwrap_or([0.0; 4]),
        view_projection: draw.camera.view_projection.to_cols_array_2d(),
        world: world.to_cols_array_2d(),
        normal: world.inverse().transpose().to_cols_array_2d(),
        base_color: factors.base_color,
        surface: [
            factors.metallic,
            factors.roughness,
            surface.alpha_cutoff.unwrap_or(-1.0),
            f32::from(draw.sidedness.resolve(surface.double_sided)),
        ],
        ambient: ambient.extend(0.0).to_array(),
        direction: direction.extend(world.determinant().signum()).to_array(),
        light: light.extend(0.0).to_array(),
        eye: draw
            .camera
            .position
            .extend(draw.fog.height_falloff)
            .to_array(),
        light_view_projection: draw.light_view_projection.to_cols_array_2d(),
        shadow: [
            draw.shadows.bias,
            f32::from(draw.shadows.enabled && lighting.directional_intensity > 0.0),
            0.0,
            0.0,
        ],
        fog_color: [
            draw.fog.color[0],
            draw.fog.color[1],
            draw.fog.color[2],
            f32::from(draw.fog.enabled),
        ],
        emissive: [
            factors.emissive[0],
            factors.emissive[1],
            factors.emissive[2],
            0.0,
        ],
        fog_params: [
            draw.fog.start,
            draw.fog.distance,
            draw.fog.density,
            draw.fog.height,
        ],
    }
}

fn bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
    texture: &Texture2D,
    sampler: &wgpu::Sampler,
    shadow_map: &ShadowMap,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Imported model draw"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(texture.view()),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(shadow_map.view()),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(shadow_map.sampler()),
            },
        ],
    })
}

fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    asset: &Arc<RenderModel>,
) -> Result<CachedModel, ModelRenderError> {
    let limit = device.limits();
    let mut meshes = Vec::new();
    for mesh in &asset.data().meshes {
        let mut primitives = Vec::new();
        for primitive in mesh {
            let vertices = bytemuck::cast_slice(&primitive.vertices);
            let (indices, format): (&[u8], _) = match &primitive.indices {
                ModelIndexData::U16(v) => (bytemuck::cast_slice(v), wgpu::IndexFormat::Uint16),
                ModelIndexData::U32(v) => (bytemuck::cast_slice(v), wgpu::IndexFormat::Uint32),
            };
            if vertices.len() as u64 > limit.max_buffer_size
                || indices.len() as u64 > limit.max_buffer_size
            {
                return Err(ModelRenderError::DeviceLimit("geometry buffer size".into()));
            }
            let count = u32::try_from(
                indices.len()
                    / if format == wgpu::IndexFormat::Uint16 {
                        2
                    } else {
                        4
                    },
            )
            .map_err(|_| ModelRenderError::DeviceLimit("index count".into()))?;
            primitives.push(Geometry {
                vertex: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Imported vertices"),
                    contents: vertices,
                    usage: wgpu::BufferUsages::VERTEX,
                }),
                index: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Imported indices"),
                    contents: indices,
                    usage: wgpu::BufferUsages::INDEX,
                }),
                count,
                format,
            });
        }
        meshes.push(primitives);
    }
    let mut textures = Vec::new();
    let mut samplers = Vec::new();
    for image in &asset.data().images {
        if image.width > limit.max_texture_dimension_2d
            || image.height > limit.max_texture_dimension_2d
        {
            return Err(ModelRenderError::DeviceLimit("texture dimensions".into()));
        }
        textures.push(
            Texture2D::from_rgba8(
                device,
                queue,
                "Imported base color",
                image.width,
                image.height,
                &image.rgba,
            )
            .map_err(|error| ModelRenderError::InvalidData(error.to_string()))?,
        );
        samplers.push(device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Imported sampler"),
            address_mode_u: image.wrap_s,
            address_mode_v: image.wrap_t,
            mag_filter: image.mag_filter,
            min_filter: image.min_filter,
            ..Default::default()
        }));
    }
    textures.push(
        Texture2D::from_rgba8(device, queue, "Imported white", 1, 1, &[255; 4])
            .map_err(|error| ModelRenderError::InvalidData(error.to_string()))?,
    );
    samplers.push(device.create_sampler(&wgpu::SamplerDescriptor::default()));
    Ok(CachedModel {
        owner: Arc::downgrade(asset),
        meshes,
        textures,
        samplers,
    })
}
