//! Each GLB image has an sRGB color view and a linear material data view.

use std::sync::{Arc, Weak};

use wgpu::util::DeviceExt;

use crate::{ModelImage, ModelRenderError, ModelSurface, RenderModel};

#[derive(Debug)]
pub(super) struct ModelTextures {
    // Hold the allocation used as the cache key without extending the model lifetime.
    owner: Weak<RenderModel>,
    pub(super) images: Vec<ModelTexture>,
}

#[derive(Debug)]
pub(super) struct ModelTexture {
    pub(super) color: wgpu::TextureView,
    pub(super) linear: wgpu::TextureView,
    pub(super) sampler: wgpu::Sampler,
}

impl ModelTextures {
    pub(super) fn upload(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        asset: &Arc<RenderModel>,
    ) -> Result<Self, ModelRenderError> {
        let max_side = device.limits().max_texture_dimension_2d;
        let mut images = Vec::new();
        for image in &asset.data().images {
            if image.width > max_side || image.height > max_side {
                return Err(ModelRenderError::DeviceLimit("texture dimensions".into()));
            }
            images.push(ModelTexture::upload(device, queue, image));
        }
        images.push(ModelTexture::upload(
            device,
            queue,
            &ModelImage {
                width: 1,
                height: 1,
                rgba: vec![255; 4],
                wrap_s: wgpu::AddressMode::ClampToEdge,
                wrap_t: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
            },
        ));
        Ok(Self {
            owner: Arc::downgrade(asset),
            images,
        })
    }

    /// Keep image uploads across changes to the batches that share this source.
    pub(super) fn source_is_live(&self) -> bool {
        self.owner.strong_count() > 0
    }

    pub(super) fn indices(&self, surface: &ModelSurface) -> [usize; 3] {
        [
            surface.texture,
            surface.normal_texture,
            surface.metallic_roughness_texture,
        ]
        .map(|index| index.unwrap_or(self.images.len() - 1))
    }
}

impl ModelTexture {
    fn upload(device: &wgpu::Device, queue: &wgpu::Queue, image: &ModelImage) -> Self {
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("Imported GLB texture"),
                size: wgpu::Extent3d {
                    width: image.width,
                    height: image.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &image.rgba,
        );
        Self {
            color: texture.create_view(&wgpu::TextureViewDescriptor {
                format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
                ..Default::default()
            }),
            linear: texture.create_view(&wgpu::TextureViewDescriptor::default()),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Imported GLB sampler"),
                address_mode_u: image.wrap_s,
                address_mode_v: image.wrap_t,
                mag_filter: image.mag_filter,
                min_filter: image.min_filter,
                ..Default::default()
            }),
        }
    }
}
