//! Static, self-contained binary glTF models. No filesystem or GPU ownership.

mod geometry;
mod parse;
#[cfg(test)]
mod tests;

use super::TextureAsset;

/// A reusable model, retaining authored node identities and local matrices.
#[derive(Clone, Debug)]
pub struct ModelAsset {
    pub nodes: Vec<ModelNode>,
    pub roots: Vec<usize>,
    pub meshes: Vec<ModelMesh>,
    pub materials: Vec<ModelMaterial>,
    pub textures: Vec<ModelTexture>,
    /// Features intentionally ignored while importing a static model.
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ModelNode {
    pub name: Option<String>,
    /// Column-major glTF local matrix, already Y-up.
    pub local_transform: [[f32; 4]; 4],
    pub children: Vec<usize>,
    pub mesh: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct ModelMesh {
    pub name: Option<String>,
    pub primitives: Vec<ModelPrimitive>,
}

#[derive(Clone, Debug)]
pub struct ModelPrimitive {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: ModelIndices,
    /// None means the glTF default material.
    pub material: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelIndices {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

impl ModelIndices {
    pub fn to_u32(&self) -> Vec<u32> {
        match self {
            Self::U16(indices) => indices.iter().copied().map(u32::from).collect(),
            Self::U32(indices) => indices.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelMaterial {
    pub name: Option<String>,
    /// Linear RGBA, as specified by glTF (not sRGB bytes).
    pub base_color: [f32; 4],
    pub base_color_texture: Option<usize>,
    pub metallic: f32,
    pub roughness: f32,
    pub double_sided: bool,
    pub alpha_cutoff: Option<f32>,
}

impl Default for ModelMaterial {
    fn default() -> Self {
        Self {
            name: None,
            base_color: [1.0; 4],
            base_color_texture: None,
            metallic: 1.0,
            roughness: 1.0,
            double_sided: false,
            alpha_cutoff: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ModelTexture {
    pub image: TextureAsset,
    /// glTF sampler enums, retained for GPU upload.
    pub wrap_s: u32,
    pub wrap_t: u32,
    pub mag_filter: Option<u32>,
    pub min_filter: Option<u32>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ModelAssetDecoder;

impl super::AssetDecoder for ModelAssetDecoder {
    type Asset = ModelAsset;

    fn decode(&self, bytes: crate::AssetBytes) -> Result<ModelAsset, super::AssetDecodeError> {
        parse::decode(&bytes)
    }
}
