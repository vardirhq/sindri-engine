//! What a sprite batch binds: its uniform, its texture and normal map, and the
//! frame's 2D lights.

use crate::light2d::Lights2dGpu;
use crate::{Texture2D, TextureId, TextureRegistry};

/// Which bind group a batch needs: one per texture and normal map pair.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct BindingKey {
    pub(super) texture: TextureId,
    pub(super) normal_map: Option<TextureId>,
}

/// A normal pointing straight out of the picture, for batches without a map.
pub(super) fn flat_normal(device: &wgpu::Device, queue: &wgpu::Queue) -> Texture2D {
    Texture2D::from_rgba8(
        device,
        queue,
        "Sindri flat normal",
        1,
        1,
        &[128, 128, 255, 255],
    )
    .expect("one texel is a valid texture")
}

pub(super) fn create_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
    lights: &Lights2dGpu,
    registry: &TextureRegistry,
    key: BindingKey,
    flat_normal: &Texture2D,
) -> wgpu::BindGroup {
    let resolved = registry.get(key.texture);
    let normal = key.normal_map.map_or(flat_normal, |id| registry.get(id));
    let [light_uniform, shadow_map] = lights.entries();
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Sindri sprite batch bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(resolved.view()),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(resolved.sampler()),
            },
            light_uniform,
            shadow_map,
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::TextureView(normal.view()),
            },
        ],
    })
}

pub(super) fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let [light_uniform, shadow_map] = Lights2dGpu::layout_entries();
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Sindri sprite batch bind group layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                // The fragment stage reads the time.
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            texture_entry(1),
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            light_uniform,
            shadow_map,
            texture_entry(5),
        ],
    })
}

const fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}
