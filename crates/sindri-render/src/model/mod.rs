//! Immutable imported geometry and retained node hierarchy, independent of assets.

mod gpu;
mod pipeline;
#[cfg(test)]
mod tests;
mod validate;

use std::sync::Arc;

use glam::Mat4;
use thiserror::Error;

pub(crate) use gpu::ModelRenderer;
pub(crate) use gpu::{ModelDraw, ModelShadowDraw};

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Clone, Debug)]
pub enum ModelIndexData {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

#[derive(Clone, Debug)]
pub struct ModelGeometry {
    pub vertices: Vec<ModelVertex>,
    pub indices: ModelIndexData,
    pub material: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct ModelHierarchyNode {
    pub name: Option<String>,
    pub local_transform: Mat4,
    pub children: Vec<usize>,
    pub mesh: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct ModelSurface {
    pub base_color: [f32; 4],
    pub texture: Option<usize>,
    pub metallic: f32,
    pub roughness: f32,
    /// Linear RGB self-emission; finite, nonnegative HDR values are allowed.
    pub emissive: [f32; 3],
    pub double_sided: bool,
    pub alpha_cutoff: Option<f32>,
}

impl Default for ModelSurface {
    fn default() -> Self {
        Self {
            base_color: [1.0; 4],
            texture: None,
            metallic: 1.0,
            roughness: 1.0,
            emissive: [0.0; 3],
            double_sided: false,
            alpha_cutoff: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ModelImage {
    pub width: u32,
    pub height: u32,
    /// sRGB RGBA bytes; glTF UVs are sampled without a vertical flip.
    pub rgba: Vec<u8>,
    pub wrap_s: wgpu::AddressMode,
    pub wrap_t: wgpu::AddressMode,
    pub mag_filter: wgpu::FilterMode,
    pub min_filter: wgpu::FilterMode,
}

/// Input at the asset/renderer boundary. One mesh contains multiple primitives.
#[derive(Clone, Debug, Default)]
pub struct ModelData {
    pub nodes: Vec<ModelHierarchyNode>,
    pub roots: Vec<usize>,
    pub meshes: Vec<Vec<ModelGeometry>>,
    pub surfaces: Vec<ModelSurface>,
    pub images: Vec<ModelImage>,
}

/// Validated immutable resource. Share one `Arc<RenderModel>` across instances.
#[derive(Debug)]
pub struct RenderModel {
    data: ModelData,
    image_owner: Option<Arc<Self>>,
    instances: Vec<ModelNodeInstance>,
    /// Model-space box around every drawn vertex, in each node's placement.
    bounds: Option<(glam::Vec3, glam::Vec3)>,
}

#[derive(Clone, Copy, Debug)]
pub struct ModelNodeInstance {
    pub node: usize,
    pub mesh: usize,
    /// Authored hierarchy accumulated in model space, without axis conversion.
    pub transform: Mat4,
}

impl RenderModel {
    pub fn new(data: ModelData) -> Result<Self, ModelRenderError> {
        Self::build(data, None)
    }

    /// Builds new geometry that uses the original model's images and samplers.
    /// Surface texture indices address the source's images.
    pub fn with_shared_images(
        data: ModelData,
        source: &Arc<Self>,
    ) -> Result<Self, ModelRenderError> {
        if !data.images.is_empty() {
            return Err(ModelRenderError::InvalidData(
                "a shared-image model cannot supply its own images".into(),
            ));
        }
        Self::build(data, Some(Arc::clone(Self::image_owner(source))))
    }

    fn build(data: ModelData, image_owner: Option<Arc<Self>>) -> Result<Self, ModelRenderError> {
        let image_count = image_owner
            .as_ref()
            .map_or(data.images.len(), |source| source.data.images.len());
        validate::data(&data, image_count)?;
        let instances = validate::hierarchy(&data)?;
        let bounds = model_bounds(&data, &instances);
        Ok(Self {
            data,
            image_owner,
            instances,
            bounds,
        })
    }

    fn image_owner(model: &Arc<Self>) -> &Arc<Self> {
        model.image_owner.as_ref().unwrap_or(model)
    }

    /// The model-space box around everything the model draws, each mesh
    /// placed by its node exactly as the renderer places it: what a caller
    /// tests against a view to skip a model nothing of which can be seen.
    /// `None` for a model with no vertices.
    #[must_use]
    pub const fn bounds(&self) -> Option<(glam::Vec3, glam::Vec3)> {
        self.bounds
    }

    pub const fn data(&self) -> &ModelData {
        &self.data
    }

    pub fn instances(&self) -> &[ModelNodeInstance] {
        &self.instances
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ModelRenderError {
    #[error("invalid imported model: {0}")]
    InvalidData(String),
    #[error("imported model transform must be finite and invertible")]
    InvalidTransform,
    #[error("imported model exceeds GPU limits: {0}")]
    DeviceLimit(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModelCacheStats {
    pub resident_models: usize,
    pub uploads: u64,
    pub draws: u64,
}

fn model_bounds(
    data: &ModelData,
    instances: &[ModelNodeInstance],
) -> Option<(glam::Vec3, glam::Vec3)> {
    let mut bounds: Option<(glam::Vec3, glam::Vec3)> = None;
    for instance in instances {
        for primitive in &data.meshes[instance.mesh] {
            for vertex in &primitive.vertices {
                let point = instance
                    .transform
                    .transform_point3(glam::Vec3::from_array(vertex.position));
                bounds = Some(bounds.map_or((point, point), |(min, max)| {
                    (min.min(point), max.max(point))
                }));
            }
        }
    }
    bounds
}
