//! Imported casters reuse color geometry, hierarchy, images and sampler policy.

use super::{CachedModel, ModelRenderer};
use crate::{ModelRenderError, ModelSurface, ModelVertex, RenderModel, shadow::ShadowMap};
use glam::Mat4;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug)]
pub(super) struct ShadowRenderer {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    slots: Vec<Slot>,
    next: usize,
}

#[derive(Debug)]
struct Slot {
    uniform: wgpu::Buffer,
    binding: Option<((usize, usize), wgpu::BindGroup)>,
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Uniform {
    light_world: [[f32; 4]; 4],
    base_color: [f32; 4],
    surface: [f32; 4],
}

#[derive(Clone, Copy)]
pub(crate) struct ModelShadowDraw<'a> {
    pub world: Mat4,
    pub projection: Mat4,
    pub target: &'a wgpu::TextureView,
    pub material: Option<crate::MaterialFactors>,
}

impl ShadowRenderer {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        // The depth-only shader uses color/alpha bindings, never the map it
        // writes. Sharing the receiving layout would create a read/write hazard.
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Imported model shadow layout"),
            entries: &super::super::pipeline::layout_entries()[..3],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Imported model shadow shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shadow.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Imported model shadow pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Imported model shadow pipeline"), layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader, entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ModelVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
                })], compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: Some("fs_main"), targets: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: ShadowMap::FORMAT, depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 },
            }),
            multisample: wgpu::MultisampleState::default(), multiview_mask: None, cache: None,
        });
        Self {
            pipeline,
            layout,
            slots: Vec::new(),
            next: 0,
        }
    }

    pub(super) fn begin_submission(&mut self, cache: &HashMap<usize, CachedModel>) {
        self.next = 0;
        for slot in &mut self.slots {
            if slot
                .binding
                .as_ref()
                .is_some_and(|((key, _), _)| !cache.contains_key(key))
            {
                slot.binding = None;
            }
        }
    }

    fn reserve(&mut self, device: &wgpu::Device) -> usize {
        if self.next == self.slots.len() {
            self.slots.push(Slot {
                uniform: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Imported model shadow uniform"),
                    size: std::mem::size_of::<Uniform>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                binding: None,
            });
        }
        let index = self.next;
        self.next += 1;
        index
    }
}

impl ModelRenderer {
    pub(crate) fn encode_shadow(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        asset: &Arc<RenderModel>,
        draw: ModelShadowDraw<'_>,
    ) -> Result<(), ModelRenderError> {
        super::validate::transform(draw.world)?;
        let key = self.ensure_cached(device, queue, asset)?;
        let cached = &self.cache[&key];
        let fallback = ModelSurface::default();
        for instance in asset.instances() {
            let world = draw.world * instance.transform;
            super::validate::transform(world)?;
            for (index, primitive) in asset.data().meshes[instance.mesh].iter().enumerate() {
                let surface = primitive
                    .material
                    .map_or(&fallback, |i| &asset.data().surfaces[i]);
                let texture = surface.texture.unwrap_or(cached.textures.len() - 1);
                // Reserve independent shadow slots: queue writes occur before
                // submission, so color writes must never overwrite these.
                let next = self.shadow.reserve(device);
                let layout = &self.shadow.layout;
                let slot = &mut self.shadow.slots[next];
                queue.write_buffer(
                    &slot.uniform,
                    0,
                    bytemuck::bytes_of(&Uniform {
                        light_world: (draw.projection * world).to_cols_array_2d(),
                        base_color: draw
                            .material
                            .map_or(surface.base_color, |factors| factors.base_color),
                        surface: [
                            surface.alpha_cutoff.unwrap_or(-1.0),
                            f32::from(surface.double_sided),
                            world.determinant().signum(),
                            0.0,
                        ],
                    }),
                );
                if slot
                    .binding
                    .as_ref()
                    .is_none_or(|(id, _)| *id != (key, texture))
                {
                    slot.binding = Some((
                        (key, texture),
                        binding(device, layout, &slot.uniform, cached, texture),
                    ));
                }
                let geometry = &cached.meshes[instance.mesh][index];
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Imported model shadow"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: draw.target,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    ..Default::default()
                });
                pass.set_pipeline(&self.shadow.pipeline);
                if let Some((_, group)) = &slot.binding {
                    pass.set_bind_group(0, group, &[]);
                }
                pass.set_vertex_buffer(0, geometry.vertex.slice(..));
                pass.set_index_buffer(geometry.index.slice(..), geometry.format);
                pass.draw_indexed(0..geometry.count, 0, 0..1);
                self.draw_stats.record_draw(std::mem::size_of::<Uniform>());
            }
        }
        Ok(())
    }
}

fn binding(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
    cached: &CachedModel,
    texture: usize,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Imported model shadow draw"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(cached.textures[texture].view()),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&cached.samplers[texture]),
            },
        ],
    })
}
