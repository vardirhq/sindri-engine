//! Immutable textured geometry shared by frames, views and shadow/color draws.

use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};
use thiserror::Error;

use crate::{MeshBuffers, TexturedMeshCacheStats, TexturedVertex};

/// Checked geometry in model space. A new resource represents a new revision;
/// transforms, textures and materials remain per-draw state.
#[derive(Debug)]
pub struct RenderMesh {
    vertices: Vec<TexturedVertex>,
    indices: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum MeshGeometryError {
    #[error("textured mesh exceeds 1,048,576 vertices or 3,145,728 indices")]
    TooLarge,
    #[error("textured mesh vertex attributes must be finite")]
    NonFiniteVertex,
    #[error("textured mesh indices must reference vertices and form triangles")]
    InvalidIndices,
}

impl RenderMesh {
    /// Validates immutable geometry before it reaches a GPU. The fixed budget
    /// keeps each buffer below baseline WebGPU device limits.
    pub fn new(
        vertices: Vec<TexturedVertex>,
        indices: Vec<u32>,
    ) -> Result<Self, MeshGeometryError> {
        if vertices.len() > 1_048_576 || indices.len() > 3_145_728 {
            return Err(MeshGeometryError::TooLarge);
        }
        if vertices.iter().any(|v| {
            !v.position
                .iter()
                .chain(&v.uv)
                .chain(&v.normal)
                .chain(std::iter::once(&v.ambient_occlusion))
                .all(|x| x.is_finite())
        }) {
            return Err(MeshGeometryError::NonFiniteVertex);
        }
        if !indices.len().is_multiple_of(3)
            || indices
                .iter()
                .any(|i| !usize::try_from(*i).is_ok_and(|i| i < vertices.len()))
        {
            return Err(MeshGeometryError::InvalidIndices);
        }
        Ok(Self { vertices, indices })
    }

    #[must_use]
    pub fn vertices(&self) -> &[TexturedVertex] {
        &self.vertices
    }

    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }
}

#[derive(Debug)]
struct Entry {
    // A weak owner prevents address reuse until this cache entry is evicted.
    owner: Weak<RenderMesh>,
    buffers: MeshBuffers,
}

#[derive(Debug, Default)]
pub(crate) struct RetainedMeshes {
    entries: HashMap<usize, Entry>,
    stats: TexturedMeshCacheStats,
}

impl RetainedMeshes {
    pub(crate) fn begin_submission(&mut self) {
        let before = self.entries.len();
        self.entries
            .retain(|_, entry| entry.owner.strong_count() > 0);
        self.stats.releases += u64::try_from(before - self.entries.len()).unwrap_or(u64::MAX);
    }

    pub(crate) fn resolve(
        &mut self,
        device: &wgpu::Device,
        geometry: &Arc<RenderMesh>,
    ) -> Option<&MeshBuffers> {
        if geometry.indices.is_empty() {
            return None;
        }
        let key = Arc::as_ptr(geometry) as usize;
        if let std::collections::hash_map::Entry::Vacant(entry) = self.entries.entry(key) {
            entry.insert(Entry {
                owner: Arc::downgrade(geometry),
                buffers: MeshBuffers::new_u32(
                    device,
                    "Sindri retained textured mesh",
                    &geometry.vertices,
                    &geometry.indices,
                ),
            });
            self.stats.installs += 1;
            self.stats.buffer_uploads += 1;
        } else {
            self.stats.reused_draws += 1;
        }
        self.entries.get(&key).map(|entry| &entry.buffers)
    }

    pub(crate) fn stats(&self) -> TexturedMeshCacheStats {
        TexturedMeshCacheStats {
            entries: self.entries.len(),
            ..self.stats
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_geometry_is_rejected_before_upload() {
        let vertex = TexturedVertex::new([0.0; 3], [0.0; 2]);
        assert_eq!(
            RenderMesh::new(vec![vertex], vec![0]).unwrap_err(),
            MeshGeometryError::InvalidIndices
        );
        assert_eq!(
            RenderMesh::new(vec![vertex], vec![0, 1, 0]).unwrap_err(),
            MeshGeometryError::InvalidIndices
        );
        let mut invalid = vertex;
        invalid.uv[0] = f32::INFINITY;
        assert_eq!(
            RenderMesh::new(vec![invalid], vec![]).unwrap_err(),
            MeshGeometryError::NonFiniteVertex
        );
        assert!(RenderMesh::new(vec![], vec![]).is_ok());
        for attribute in 0..4 {
            let mut invalid = vertex;
            match attribute {
                0 => invalid.position[0] = f32::NAN,
                1 => invalid.uv[0] = f32::NAN,
                2 => invalid.normal[0] = f32::NAN,
                _ => invalid.ambient_occlusion = f32::NAN,
            }
            assert_eq!(
                RenderMesh::new(vec![invalid], vec![]).unwrap_err(),
                MeshGeometryError::NonFiniteVertex
            );
        }
        assert_eq!(
            RenderMesh::new(vec![vertex; 1_048_577], vec![]).unwrap_err(),
            MeshGeometryError::TooLarge
        );
        assert_eq!(
            RenderMesh::new(vec![vertex], vec![0; 3_145_729]).unwrap_err(),
            MeshGeometryError::TooLarge
        );
    }
}
