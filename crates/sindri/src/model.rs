//! Asset-to-render conversion at the public facade, outside both foundations.

use std::sync::Arc;

use sindri_assets::{ModelAsset, ModelIndices};
use sindri_render::{
    ModelData, ModelGeometry, ModelHierarchyNode, ModelImage, ModelIndexData, ModelRenderError,
    ModelSurface, ModelVertex, RenderModel,
};

#[derive(Debug)]
pub struct PreparedModel {
    pub resource: Arc<RenderModel>,
    /// Hosts must report these alongside the logical asset ID.
    pub warnings: Vec<String>,
}

/// Converts one decoded static asset. Share the returned resource across entities.
pub fn prepare(asset: ModelAsset) -> Result<PreparedModel, ModelRenderError> {
    let mut meshes = Vec::new();
    for mesh in asset.meshes {
        let mut primitives = Vec::new();
        for primitive in mesh.primitives {
            if primitive.positions.len() != primitive.normals.len()
                || primitive.positions.len() != primitive.uvs.len()
            {
                return Err(ModelRenderError::InvalidData(
                    "model vertex attribute counts differ".into(),
                ));
            }
            let vertices = primitive
                .positions
                .into_iter()
                .zip(primitive.normals)
                .zip(primitive.uvs)
                .map(|((position, normal), uv)| ModelVertex {
                    position,
                    normal,
                    uv,
                })
                .collect();
            primitives.push(ModelGeometry {
                vertices,
                indices: match primitive.indices {
                    ModelIndices::U16(indices) => ModelIndexData::U16(indices),
                    ModelIndices::U32(indices) => ModelIndexData::U32(indices),
                },
                material: primitive.material,
            });
        }
        meshes.push(primitives);
    }
    let nodes = asset
        .nodes
        .into_iter()
        .map(|node| ModelHierarchyNode {
            name: node.name,
            local_transform: glam::Mat4::from_cols_array_2d(&node.local_transform),
            children: node.children,
            mesh: node.mesh,
        })
        .collect();
    let surfaces = asset
        .materials
        .into_iter()
        .map(|material| ModelSurface {
            base_color: material.base_color,
            texture: material.base_color_texture,
            metallic: material.metallic,
            roughness: material.roughness,
            double_sided: material.double_sided,
            alpha_cutoff: material.alpha_cutoff,
        })
        .collect();
    let images = asset
        .textures
        .into_iter()
        .map(|texture| {
            Ok(ModelImage {
                width: texture.image.width(),
                height: texture.image.height(),
                rgba: texture.image.rgba8().to_vec(),
                wrap_s: wrap(texture.wrap_s)?,
                wrap_t: wrap(texture.wrap_t)?,
                mag_filter: filter(texture.mag_filter)?,
                min_filter: filter(texture.min_filter)?,
            })
        })
        .collect::<Result<Vec<_>, ModelRenderError>>()?;
    Ok(PreparedModel {
        resource: Arc::new(RenderModel::new(ModelData {
            nodes,
            roots: asset.roots,
            meshes,
            surfaces,
            images,
        })?),
        warnings: asset.warnings,
    })
}

fn wrap(value: u32) -> Result<sindri_gpu::wgpu::AddressMode, ModelRenderError> {
    match value {
        10497 => Ok(sindri_gpu::wgpu::AddressMode::Repeat),
        33071 => Ok(sindri_gpu::wgpu::AddressMode::ClampToEdge),
        33648 => Ok(sindri_gpu::wgpu::AddressMode::MirrorRepeat),
        _ => Err(ModelRenderError::InvalidData(format!(
            "unsupported texture wrap mode {value}"
        ))),
    }
}

fn filter(value: Option<u32>) -> Result<sindri_gpu::wgpu::FilterMode, ModelRenderError> {
    match value {
        Some(9728 | 9984 | 9986) => Ok(sindri_gpu::wgpu::FilterMode::Nearest),
        None | Some(9729 | 9985 | 9987) => Ok(sindri_gpu::wgpu::FilterMode::Linear),
        Some(value) => Err(ModelRenderError::InvalidData(format!(
            "unsupported texture filter {value}"
        ))),
    }
}
