//! Bake imported block geometry into section-local model batches.
use std::{collections::BTreeMap, sync::Arc};

use glam::{Mat4, Vec3};
use sindri_core::AssetId;
use sindri_render::{
    ModelData, ModelGeometry, ModelHierarchyNode, ModelIndexData, ModelRenderError, ModelVertex,
    RenderModel,
};
use sindri_voxel::VoxelId;

use super::{SceneExtractError, voxel_appearance::Appearance};

#[derive(Debug)]
struct BoundModel {
    id: AssetId,
    asset: Arc<RenderModel>,
}

impl PartialEq for BoundModel {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && Arc::ptr_eq(&self.asset, &other.asset)
    }
}

#[derive(Debug, Default, PartialEq)]
pub(super) struct VoxelModels(BTreeMap<u16, BoundModel>);

impl VoxelModels {
    pub(super) fn resolve(
        appearance: &Appearance,
        bindings: &BTreeMap<AssetId, Arc<RenderModel>>,
        tolerant: bool,
    ) -> Result<(Self, Option<SceneExtractError>), SceneExtractError> {
        let mut models = BTreeMap::new();
        let mut diagnostic = None;
        if let Appearance::Blocks(blocks) = appearance {
            for (voxel, _, block) in blocks {
                let Some(id) = &block.model else { continue };
                match checked_model(id, bindings) {
                    Ok(model) => {
                        models.insert(
                            *voxel,
                            BoundModel {
                                id: id.clone(),
                                asset: Arc::clone(model),
                            },
                        );
                    }
                    Err(error) if tolerant => {
                        diagnostic.get_or_insert(error);
                    }
                    Err(error) => return Err(error),
                }
            }
        }
        Ok((Self(models), diagnostic))
    }

    pub(super) fn contains(&self, voxel: u16) -> bool {
        self.0.contains_key(&voxel)
    }

    pub(super) fn compile(
        &self,
        cells: &[([u8; 3], VoxelId)],
    ) -> Result<Vec<Arc<RenderModel>>, ModelRenderError> {
        let mut locations: BTreeMap<_, (&Arc<RenderModel>, Vec<Vec3>)> = BTreeMap::new();
        for (cell, voxel) in cells {
            let model = &self.0[&voxel.value()];
            let (_, offsets) = locations
                .entry(&model.id)
                .or_insert_with(|| (&model.asset, Vec::new()));
            offsets.push(Vec3::from_array(cell.map(f32::from)));
        }
        locations
            .into_values()
            .map(|(asset, offsets)| batch(asset, &offsets).map(Arc::new))
            .collect()
    }
}

fn checked_model<'a>(
    id: &AssetId,
    bindings: &'a BTreeMap<AssetId, Arc<RenderModel>>,
) -> Result<&'a Arc<RenderModel>, SceneExtractError> {
    let model = bindings
        .get(id)
        .ok_or_else(|| SceneExtractError::MissingModel(id.clone()))?;
    // Allow accumulated node transforms a small rounding tolerance.
    let in_cell = model.bounds().is_some_and(|(min, max)| {
        min.cmpge(Vec3::splat(-1e-5)).all() && max.cmple(Vec3::splat(1.0 + 1e-5)).all()
    });
    let opaque = model
        .data()
        .surfaces
        .iter()
        .all(|surface| surface.alpha_cutoff.is_none());
    if !in_cell || !opaque {
        return Err(SceneExtractError::InvalidVoxelModel(id.clone()));
    }
    Ok(model)
}

#[derive(Default)]
struct PrimitiveBatch {
    vertices: Vec<ModelVertex>,
    indices: Vec<u32>,
}

fn batch(source: &Arc<RenderModel>, offsets: &[Vec3]) -> Result<RenderModel, ModelRenderError> {
    let mut groups: BTreeMap<Option<usize>, PrimitiveBatch> = BTreeMap::new();
    let data = source.data();
    for instance in source.instances() {
        let local = instance.transform;
        let normal = local.inverse().transpose();
        for primitive in &data.meshes[instance.mesh] {
            let target = groups.entry(primitive.material).or_default();
            let vertices: Vec<_> = primitive
                .vertices
                .iter()
                .map(|vertex| ModelVertex {
                    position: local
                        .transform_point3(Vec3::from_array(vertex.position))
                        .to_array(),
                    normal: normal
                        .transform_vector3(Vec3::from_array(vertex.normal))
                        .normalize_or_zero()
                        .to_array(),
                    uv: vertex.uv,
                })
                .collect();
            let mut indices = match &primitive.indices {
                ModelIndexData::U16(indices) => indices.iter().copied().map(u32::from).collect(),
                ModelIndexData::U32(indices) => indices.clone(),
            };
            if local.determinant().is_sign_negative() {
                for triangle in indices.chunks_exact_mut(3) {
                    triangle.swap(1, 2);
                }
            }
            for offset in offsets {
                append(target, &vertices, &indices, *offset)?;
            }
        }
    }
    RenderModel::with_shared_images(
        ModelData {
            nodes: vec![ModelHierarchyNode {
                name: None,
                local_transform: Mat4::IDENTITY,
                children: Vec::new(),
                mesh: Some(0),
            }],
            roots: vec![0],
            meshes: vec![
                groups
                    .into_iter()
                    .map(|(material, batch)| ModelGeometry {
                        vertices: batch.vertices,
                        indices: ModelIndexData::U32(batch.indices),
                        material,
                    })
                    .collect(),
            ],
            surfaces: data.surfaces.clone(),
            images: Vec::new(),
        },
        source,
    )
}

fn append(
    target: &mut PrimitiveBatch,
    vertices: &[ModelVertex],
    indices: &[u32],
    offset: Vec3,
) -> Result<(), ModelRenderError> {
    let origin = target.vertices.len();
    let end = origin.saturating_add(vertices.len());
    if u32::try_from(end).is_err() {
        return Err(ModelRenderError::InvalidData(
            "too many modeled block vertices in a section".into(),
        ));
    }
    let base = u32::try_from(origin).expect("the end also fits in u32");
    target
        .vertices
        .extend(vertices.iter().map(|vertex| ModelVertex {
            position: (Vec3::from_array(vertex.position) + offset).to_array(),
            ..*vertex
        }));
    target
        .indices
        .extend(indices.iter().map(|index| base + index));
    Ok(())
}
