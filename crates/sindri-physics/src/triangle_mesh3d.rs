//! Immutable, asset-free geometry for static 3D triangle collision.

use rapier3d::prelude as r3;
use thiserror::Error;

use crate::{CollisionLayers, PhysicsMaterial, PhysicsPose3d};

/// Construction failures never expose backend errors or partially built geometry.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum TriangleMeshError3d {
    #[error("triangle collision geometry must contain vertices and triangles")]
    Empty,
    #[error("triangle collision geometry exceeds 1,000,000 vertices or triangles")]
    Limit,
    #[error("triangle collision vertex {0} must be finite and within +/-100,000,000 units")]
    Vertex(usize),
    #[error("triangle collision triangle {0} has an out-of-range vertex index")]
    Index(usize),
    #[error("triangle collision triangle {0} has degenerate or unrepresentable area")]
    Area(usize),
    #[error("triangle collision backend rejected the validated geometry")]
    Construction,
}

/// Shared validated model-space surface geometry. Cloning shares its immutable
/// acceleration structure; it does not copy triangles or rebuild the mesh.
/// This runtime resource is not a serialized scene shape or a dynamic hull.
#[derive(Clone, Debug)]
pub struct TriangleMesh3d {
    pub(crate) shape: r3::SharedShape,
}

impl TriangleMesh3d {
    /// Builds a two-sided triangle surface without welding or topology repair.
    /// Open, disconnected and nonmanifold surfaces are permitted. Counts are
    /// bounded to one million each; coordinates are bounded to keep backend
    /// f32 area arithmetic representable. Scale must be baked into vertices.
    ///
    /// # Errors
    /// Rejects empty/oversized buffers, invalid vertices/indices and triangles
    /// whose f32 cross-product squared length is zero or nonfinite.
    pub fn new(vertices: &[[f32; 3]], triangles: &[[u32; 3]]) -> Result<Self, TriangleMeshError3d> {
        if vertices.is_empty() || triangles.is_empty() {
            return Err(TriangleMeshError3d::Empty);
        }
        if vertices.len() > 1_000_000 || triangles.len() > 1_000_000 {
            return Err(TriangleMeshError3d::Limit);
        }
        for (index, vertex) in vertices.iter().enumerate() {
            if !vertex
                .iter()
                .all(|value| value.is_finite() && value.abs() <= 100_000_000.0)
            {
                return Err(TriangleMeshError3d::Vertex(index));
            }
        }
        for (index, triangle) in triangles.iter().enumerate() {
            let [a, b, c] =
                triangle.map(|vertex| usize::try_from(vertex).ok().and_then(|i| vertices.get(i)));
            let (Some(a), Some(b), Some(c)) = (a, b, c) else {
                return Err(TriangleMeshError3d::Index(index));
            };
            let a = r3::Vector::from_array(*a);
            let b = r3::Vector::from_array(*b);
            let c = r3::Vector::from_array(*c);
            let area = (b - a).cross(c - a).length_squared();
            if !area.is_finite() || area <= 0.0 {
                return Err(TriangleMeshError3d::Area(index));
            }
        }
        let vertices = vertices
            .iter()
            .map(|&point| r3::Vector::from_array(point))
            .collect();
        let shape = r3::SharedShape::trimesh(vertices, triangles.to_vec())
            .map_err(|_| TriangleMeshError3d::Construction)?;
        Ok(Self { shape })
    }
}

/// A reusable triangle surface and local collider properties. These properties
/// are validated as a complete group before any world mutation.
#[derive(Clone, Debug)]
pub struct TriangleMeshCollider3d {
    pub geometry: TriangleMesh3d,
    pub pose: PhysicsPose3d,
    pub sensor: bool,
    pub layers: CollisionLayers,
    pub material: PhysicsMaterial,
}

impl TriangleMeshCollider3d {
    pub fn new(geometry: TriangleMesh3d) -> Self {
        Self {
            geometry,
            pose: PhysicsPose3d::default(),
            sensor: false,
            layers: CollisionLayers::ALL,
            material: PhysicsMaterial {
                friction: 0.5,
                restitution: 0.0,
            },
        }
    }
}
