use crate::TexturedVertex;

const FACE_UVS: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

/// One unit across, centred on the origin: the size of a default Collider 3D
/// box, so a cube mesh and its collider match at any shared scale.
pub(super) const VERTICES: [TexturedVertex; 24] = [
    TexturedVertex::new([-0.5, -0.5, 0.5], FACE_UVS[0]).with_normal([0.0, 0.0, 1.0]),
    TexturedVertex::new([0.5, -0.5, 0.5], FACE_UVS[1]).with_normal([0.0, 0.0, 1.0]),
    TexturedVertex::new([0.5, 0.5, 0.5], FACE_UVS[2]).with_normal([0.0, 0.0, 1.0]),
    TexturedVertex::new([-0.5, 0.5, 0.5], FACE_UVS[3]).with_normal([0.0, 0.0, 1.0]),
    TexturedVertex::new([0.5, -0.5, -0.5], FACE_UVS[0]).with_normal([0.0, 0.0, -1.0]),
    TexturedVertex::new([-0.5, -0.5, -0.5], FACE_UVS[1]).with_normal([0.0, 0.0, -1.0]),
    TexturedVertex::new([-0.5, 0.5, -0.5], FACE_UVS[2]).with_normal([0.0, 0.0, -1.0]),
    TexturedVertex::new([0.5, 0.5, -0.5], FACE_UVS[3]).with_normal([0.0, 0.0, -1.0]),
    TexturedVertex::new([0.5, -0.5, 0.5], FACE_UVS[0]).with_normal([1.0, 0.0, 0.0]),
    TexturedVertex::new([0.5, -0.5, -0.5], FACE_UVS[1]).with_normal([1.0, 0.0, 0.0]),
    TexturedVertex::new([0.5, 0.5, -0.5], FACE_UVS[2]).with_normal([1.0, 0.0, 0.0]),
    TexturedVertex::new([0.5, 0.5, 0.5], FACE_UVS[3]).with_normal([1.0, 0.0, 0.0]),
    TexturedVertex::new([-0.5, -0.5, -0.5], FACE_UVS[0]).with_normal([-1.0, 0.0, 0.0]),
    TexturedVertex::new([-0.5, -0.5, 0.5], FACE_UVS[1]).with_normal([-1.0, 0.0, 0.0]),
    TexturedVertex::new([-0.5, 0.5, 0.5], FACE_UVS[2]).with_normal([-1.0, 0.0, 0.0]),
    TexturedVertex::new([-0.5, 0.5, -0.5], FACE_UVS[3]).with_normal([-1.0, 0.0, 0.0]),
    TexturedVertex::new([-0.5, 0.5, 0.5], FACE_UVS[0]).with_normal([0.0, 1.0, 0.0]),
    TexturedVertex::new([0.5, 0.5, 0.5], FACE_UVS[1]).with_normal([0.0, 1.0, 0.0]),
    TexturedVertex::new([0.5, 0.5, -0.5], FACE_UVS[2]).with_normal([0.0, 1.0, 0.0]),
    TexturedVertex::new([-0.5, 0.5, -0.5], FACE_UVS[3]).with_normal([0.0, 1.0, 0.0]),
    TexturedVertex::new([-0.5, -0.5, -0.5], FACE_UVS[0]).with_normal([0.0, -1.0, 0.0]),
    TexturedVertex::new([0.5, -0.5, -0.5], FACE_UVS[1]).with_normal([0.0, -1.0, 0.0]),
    TexturedVertex::new([0.5, -0.5, 0.5], FACE_UVS[2]).with_normal([0.0, -1.0, 0.0]),
    TexturedVertex::new([-0.5, -0.5, 0.5], FACE_UVS[3]).with_normal([0.0, -1.0, 0.0]),
];

pub(super) const INDICES: [u16; 36] = [
    0, 1, 2, 2, 3, 0, 4, 5, 6, 6, 7, 4, 8, 9, 10, 10, 11, 8, 12, 13, 14, 14, 15, 12, 16, 17, 18,
    18, 19, 16, 20, 21, 22, 22, 23, 20,
];

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn cube_face_normals_are_unit_outward_and_match_winding() {
        for (face, triangles) in VERTICES.chunks_exact(4).zip(INDICES.chunks_exact(6)) {
            let normal = Vec3::from_array(face[0].normal);
            assert!((normal.length() - 1.0).abs() < f32::EPSILON);
            for vertex in face {
                assert!((Vec3::from_array(vertex.normal) - normal).length() < f32::EPSILON);
                assert!(Vec3::from_array(vertex.position).dot(normal) > 0.49);
            }
            let [a, b, c] = triangles[..3].try_into().unwrap();
            let position = |index: u16| Vec3::from_array(VERTICES[usize::from(index)].position);
            let geometric = (position(b) - position(a))
                .cross(position(c) - position(a))
                .normalize();
            assert!(geometric.dot(normal) > 0.99);
        }
    }
}
