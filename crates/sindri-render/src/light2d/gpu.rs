//! The frame's 2D lights on the GPU: one uniform and one shadow texture.
//!
//! Each renderer that lights 2D content keeps its own pair, created with the
//! renderer so every bind group made from it stays valid, and rewritten when
//! the frame's lighting changes.

use super::{Lighting2d, MAX_LIGHTS_2D, SHADOW_BINS_2D, SHADOW_SAMPLES_2D};

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniform {
    position: [f32; 4],
    color: [f32; 4],
    shadow: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightsUniform {
    ambient: [f32; 4],
    header: [f32; 4],
    lights: [LightUniform; MAX_LIGHTS_2D],
}

#[allow(clippy::cast_possible_truncation)]
const BINS: u32 = SHADOW_BINS_2D as u32;
#[allow(clippy::cast_possible_truncation)]
const ROWS: u32 = (MAX_LIGHTS_2D * SHADOW_SAMPLES_2D) as u32;

#[derive(Debug)]
pub(crate) struct Lights2dGpu {
    uniform: wgpu::Buffer,
    shadow: wgpu::Texture,
    shadow_view: wgpu::TextureView,
    /// What the buffers hold, so an unchanged frame writes nothing.
    written: Written,
}

/// What a renderer last uploaded.
#[derive(Debug, PartialEq)]
enum Written {
    Nothing,
    Off,
    Lit(Lighting2d),
}

impl Lights2dGpu {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sindri 2D lights"),
            size: std::mem::size_of::<LightsUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shadow = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Sindri 2D shadow maps"),
            size: wgpu::Extent3d {
                width: BINS,
                height: ROWS,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let shadow_view = shadow.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            uniform,
            shadow,
            shadow_view,
            written: Written::Nothing,
        }
    }

    /// Uploads `lighting` unless it is what the buffers already hold.
    pub(crate) fn write(&mut self, queue: &wgpu::Queue, lighting: Option<&Lighting2d>) {
        let unchanged = match (&self.written, lighting) {
            (Written::Off, None) => true,
            (Written::Lit(written), Some(lighting)) => written == lighting,
            _ => false,
        };
        if unchanged {
            return;
        }
        let mut uniform = LightsUniform {
            ambient: [1.0, 1.0, 1.0, 0.0],
            header: [0.0; 4],
            lights: [LightUniform {
                position: [0.0; 4],
                color: [0.0; 4],
                shadow: [0.0; 4],
            }; MAX_LIGHTS_2D],
        };
        let mut rows = vec![0.0_f32; SHADOW_BINS_2D * MAX_LIGHTS_2D * SHADOW_SAMPLES_2D];
        if let Some(lighting) = lighting {
            let [r, g, b] = lighting.ambient;
            uniform.ambient = [r, g, b, 1.0];
            let lights = &lighting.lights[..lighting.lights.len().min(MAX_LIGHTS_2D)];
            #[allow(clippy::cast_precision_loss)]
            let count = lights.len() as f32;
            uniform.header[0] = count;
            for (index, light) in lights.iter().enumerate() {
                let [r, g, b] = light.color;
                uniform.lights[index] = LightUniform {
                    position: [
                        light.position[0],
                        light.position[1],
                        light.height,
                        light.radius,
                    ],
                    color: [r, g, b, light.falloff],
                    shadow: [
                        f32::from(u8::from(light.shadow.is_some())),
                        light.shadow.as_ref().map_or(0.0, |shadow| shadow.reach),
                        light
                            .shadow
                            .as_ref()
                            .map_or(0.0, |shadow| shadow.source_radius),
                        0.0,
                    ],
                };
                if let Some(shadow) = &light.shadow {
                    let row = &mut rows[index * SHADOW_BINS_2D * SHADOW_SAMPLES_2D
                        ..(index + 1) * SHADOW_BINS_2D * SHADOW_SAMPLES_2D];
                    for (into, from) in row.iter_mut().zip(shadow.map.iter()) {
                        *into = *from;
                    }
                }
            }
        }
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uniform));
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.shadow,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&rows),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(BINS * 4),
                rows_per_image: Some(ROWS),
            },
            wgpu::Extent3d {
                width: BINS,
                height: ROWS,
                depth_or_array_layers: 1,
            },
        );
        self.written = lighting.map_or(Written::Off, |lighting| Written::Lit(lighting.clone()));
    }

    /// The two bind group entries, at the bindings `light2d.wgsl` declares.
    pub(crate) fn entries(&self) -> [wgpu::BindGroupEntry<'_>; 2] {
        [
            wgpu::BindGroupEntry {
                binding: 3,
                resource: self.uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(&self.shadow_view),
            },
        ]
    }

    /// The two layout entries matching [`Self::entries`].
    pub(crate) const fn layout_entries() -> [wgpu::BindGroupLayoutEntry; 2] {
        [
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
        ]
    }
}
