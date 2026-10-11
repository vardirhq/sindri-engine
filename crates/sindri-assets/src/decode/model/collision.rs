//! Bounded CPU triangles in model space, ready for a separate physics adapter.
mod matrix;
#[cfg(test)]
mod tests;

use super::{ModelAsset, ModelPrimitive};
use std::{collections::BTreeSet, ops::Range};
use thiserror::Error;

/// Bounds on expanded geometry, including repeated mesh instances.
#[derive(Clone, Copy, Debug)]
pub struct ModelCollisionLimits {
    pub nodes: usize,
    pub vertices: usize,
    pub triangles: usize,
}

impl Default for ModelCollisionLimits {
    fn default() -> Self {
        Self {
            nodes: 65_536,
            vertices: 1_000_000,
            triangles: 1_000_000,
        }
    }
}

/// Owned selected-scene triangles. Entity transforms are applied by the host.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelCollisionMesh {
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub parts: Vec<ModelCollisionPart>,
}

/// Source identity and ranges for one instantiated mesh primitive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelCollisionPart {
    pub node: usize,
    pub mesh: usize,
    pub primitive: usize,
    pub vertices: Range<usize>,
    pub triangles: Range<usize>,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ModelCollisionError {
    #[error("selected model collision exceeds its hierarchy node limit")]
    NodeLimit,
    #[error("collision hierarchy has an invalid or repeated node {0}")]
    Hierarchy(usize),
    #[error("collision node {0} has a nonfinite, singular or non-affine transform")]
    Transform(usize),
    #[error("collision node {node}, mesh {mesh}, primitive {primitive}: {reason}")]
    Geometry {
        node: usize,
        mesh: usize,
        primitive: usize,
        reason: &'static str,
    },
    #[error("expanded model collision exceeds its vertex limit or u32 index range")]
    VertexLimit,
    #[error("expanded model collision exceeds its triangle limit")]
    TriangleLimit,
    #[error("selected model scene has no collision triangles")]
    Empty,
}

impl ModelAsset {
    /// Expand selected roots in authored order, baking accumulated node matrices.
    ///
    /// Reflections reverse triangle indices to preserve authored orientation.
    /// Degenerate triangles and invalid mutable asset data return typed errors.
    /// No material filtering, welding, hull generation or physics body is implied.
    pub fn collision_mesh(
        &self,
        limits: ModelCollisionLimits,
    ) -> Result<ModelCollisionMesh, ModelCollisionError> {
        let mut result = ModelCollisionMesh {
            positions: Vec::new(),
            triangles: Vec::new(),
            parts: Vec::new(),
        };
        if self.roots.len() > limits.nodes {
            return Err(ModelCollisionError::NodeLimit);
        }
        let mut seen = BTreeSet::new();
        let mut pending: Vec<_> = self
            .roots
            .iter()
            .rev()
            .map(|&node| (node, matrix::IDENTITY))
            .collect();
        while let Some((index, parent)) = pending.pop() {
            let node = self
                .nodes
                .get(index)
                .ok_or(ModelCollisionError::Hierarchy(index))?;
            if !seen.insert(index) {
                return Err(ModelCollisionError::Hierarchy(index));
            }
            if seen
                .len()
                .checked_add(pending.len())
                .and_then(|count| count.checked_add(node.children.len()))
                .is_none_or(|count| count > limits.nodes)
            {
                return Err(ModelCollisionError::NodeLimit);
            }
            let transform = matrix::compose(parent, node.local_transform)
                .ok_or(ModelCollisionError::Transform(index))?;
            if let Some(mesh) = node.mesh {
                let source = self.meshes.get(mesh).ok_or(ModelCollisionError::Geometry {
                    node: index,
                    mesh,
                    primitive: 0,
                    reason: "missing mesh",
                })?;
                for (primitive, source) in source.primitives.iter().enumerate() {
                    append(
                        &mut result,
                        source,
                        transform,
                        [index, mesh, primitive],
                        limits,
                    )?;
                }
            }
            pending.extend(node.children.iter().rev().map(|&child| (child, transform)));
        }
        if result.triangles.is_empty() {
            return Err(ModelCollisionError::Empty);
        }
        Ok(result)
    }
}

fn append(
    output: &mut ModelCollisionMesh,
    source: &ModelPrimitive,
    transform: matrix::Matrix,
    [node, mesh, primitive]: [usize; 3],
    limits: ModelCollisionLimits,
) -> Result<(), ModelCollisionError> {
    let invalid = |reason| ModelCollisionError::Geometry {
        node,
        mesh,
        primitive,
        reason,
    };
    let end = output
        .positions
        .len()
        .checked_add(source.positions.len())
        .filter(|&count| count <= limits.vertices && u32::try_from(count).is_ok())
        .ok_or(ModelCollisionError::VertexLimit)?;
    // Check the borrowed index count before allocating a flattened copy.
    let count = match &source.indices {
        super::ModelIndices::U16(indices) => indices.len(),
        super::ModelIndices::U32(indices) => indices.len(),
    };
    if source.positions.is_empty() || count == 0 || !count.is_multiple_of(3) {
        return Err(invalid("empty or incomplete triangles"));
    }
    let triangle_end = output
        .triangles
        .len()
        .checked_add(count / 3)
        .filter(|&count| count <= limits.triangles)
        .ok_or(ModelCollisionError::TriangleLimit)?;
    let start = output.positions.len();
    for &position in &source.positions {
        output.positions.push(
            matrix::point(transform, position)
                .ok_or_else(|| invalid("nonfinite or overflowing vertex"))?,
        );
    }
    let indices = source.indices.to_u32();
    let triangle_start = output.triangles.len();
    for triangle in indices.chunks_exact(3) {
        let mut ids = [0_u32; 3];
        let mut points = [[0.0; 3]; 3];
        for (corner, &index) in triangle.iter().enumerate() {
            let index = usize::try_from(index).map_err(|_| invalid("index out of range"))?;
            if index >= source.positions.len() {
                return Err(invalid("index out of range"));
            }
            ids[corner] =
                u32::try_from(start + index).map_err(|_| ModelCollisionError::VertexLimit)?;
            points[corner] = output.positions[start + index];
        }
        if !matrix::has_area(points) {
            return Err(invalid("degenerate triangle"));
        }
        if matrix::determinant(transform) < 0.0 {
            ids.swap(1, 2);
        }
        output.triangles.push(ids);
    }
    output.parts.push(ModelCollisionPart {
        node,
        mesh,
        primitive,
        vertices: start..end,
        triangles: triangle_start..triangle_end,
    });
    Ok(())
}
