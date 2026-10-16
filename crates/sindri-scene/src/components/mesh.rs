//! `sindri.mesh`: built-in and authored geometry to draw.

use serde::Deserialize;
use sindri_core::SceneComponent;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MeshPrimitive {
    /// A unit cube, one unit across and centred on the entity: the size of a
    /// default Collider 3D box, so the two match at any scale.
    Cube,
    Surface,
}

/// Explicit textured geometry.
///
/// This is deliberately small: it is enough for generated terrain and imported
/// geometry to cross the scene/render boundary without teaching the renderer
/// what a terrain chunk is. Larger asset-backed meshes can replace the inline
/// representation later without changing that boundary.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(try_from = "SurfaceDocument")]
pub struct SurfaceMesh {
    #[serde(default)]
    pub vertices: Vec<[f32; 3]>,
    #[serde(default)]
    pub uvs: Vec<[f32; 2]>,
    #[serde(default)]
    pub indices: Vec<u16>,
    /// One finite, nonzero object-space normal per vertex, or flat shading.
    pub normals: Option<Vec<[f32; 3]>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct MeshComponent {
    pub primitive: MeshPrimitive,
    pub texture: String,
    #[serde(default)]
    pub layer: i32,
    #[serde(default)]
    pub surface: Option<SurfaceMesh>,
}

impl SceneComponent for MeshComponent {
    const TYPE_NAME: &'static str = "sindri.mesh";
}

#[derive(Deserialize)]
struct SurfaceDocument {
    #[serde(default)]
    vertices: Vec<[f32; 3]>,
    #[serde(default)]
    uvs: Vec<[f32; 2]>,
    #[serde(default)]
    indices: Vec<u16>,
    #[serde(default)]
    normals: Option<Vec<[f32; 3]>>,
}

impl TryFrom<SurfaceDocument> for SurfaceMesh {
    type Error = serde::de::value::Error;

    fn try_from(value: SurfaceDocument) -> Result<Self, Self::Error> {
        if value.normals.as_ref().is_some_and(|normals| {
            normals.len() != value.vertices.len()
                || normals
                    .iter()
                    .any(|n| glam::Vec3::from_array(*n).try_normalize().is_none())
        }) {
            return Err(<Self::Error as serde::de::Error>::custom(
                "surface normals must be normalizable and match the vertex count",
            ));
        }
        Ok(Self {
            vertices: value.vertices,
            uvs: value.uvs,
            indices: value.indices,
            normals: value.normals,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_normals_reject_partial_zero_and_nonfinite_input() {
        let mut document = serde_json::json!({ "primitive":"surface", "texture":"t",
            "surface": { "vertices": [[0,0,0], [1,0,0], [0,1,0]],
                "uvs": [[0,0],[1,0],[0,1]], "indices": [0,1,2] } });
        let plain: MeshComponent = serde_json::from_value(document.clone()).unwrap();
        assert!(plain.surface.unwrap().normals.is_none());
        for normals in [
            serde_json::json!([[0, 0, 1]]),
            serde_json::json!([[0, 0, 1], [0, 0, 0], [0, 0, 1]]),
            serde_json::json!([[0, 0, 1], [1e99, 0, 1], [0, 0, 1]]),
        ] {
            document["surface"]["normals"] = normals;
            assert!(serde_json::from_value::<MeshComponent>(document.clone()).is_err());
        }
        document["surface"]["normals"] = serde_json::json!([[0, 0, 2], [1, 0, 1], [0, 1, 1]]);
        assert!(serde_json::from_value::<MeshComponent>(document).is_ok());
    }
}
