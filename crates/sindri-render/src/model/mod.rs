//! Immutable imported geometry and retained node hierarchy, independent of assets.

mod gpu;
mod pipeline;
#[cfg(test)]
mod tests;
mod validate;

use glam::Mat4;
use thiserror::Error;

pub(crate) use gpu::ModelDraw;
pub(crate) use gpu::ModelRenderer;

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
    instances: Vec<ModelNodeInstance>,
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
        validate::data(&data)?;
        let instances = validate::hierarchy(&data)?;
        Ok(Self { data, instances })
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
