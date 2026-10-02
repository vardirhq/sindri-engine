use crate::{
    FaceOcclusion, RenderClass, SectionBounds, SectionCoord, VoxelCoord, VoxelFace, VoxelId,
    VoxelMaterial, VoxelMaterialSource, VoxelShape, VoxelSource, mesh_block_section,
    mesh_block_section_with_materials,
};

struct Solid;

impl VoxelSource for Solid {
    fn voxel(&self, _coord: VoxelCoord) -> VoxelId {
        VoxelId::new(1)
    }
}

struct OneSection;

impl VoxelSource for OneSection {
    fn voxel(&self, coord: VoxelCoord) -> VoxelId {
        if coord.section() == SectionCoord::new(0, 0, 0) {
            VoxelId::new(2)
        } else {
            VoxelId::AIR
        }
    }
}

struct TwoSections;

impl VoxelSource for TwoSections {
    fn voxel(&self, coord: VoxelCoord) -> VoxelId {
        if matches!(
            coord.section(),
            SectionCoord {
                x: 0 | 1,
                y: 0,
                z: 0
            }
        ) {
            VoxelId::new(3)
        } else {
            VoxelId::AIR
        }
    }
}

struct Pair {
    right: VoxelId,
}

impl VoxelSource for Pair {
    fn voxel(&self, coord: VoxelCoord) -> VoxelId {
        match (coord.x, coord.y, coord.z) {
            (0, 0, 0) => VoxelId::new(4),
            (1, 0, 0) => self.right,
            _ => VoxelId::AIR,
        }
    }
}

struct TestMaterials;

impl VoxelMaterialSource for TestMaterials {
    fn material(&self, voxel: VoxelId) -> VoxelMaterial {
        match voxel.value() {
            1 => VoxelMaterial::opaque(),
            2 => VoxelMaterial::cutout(),
            3..=5 => VoxelMaterial::transparent(),
            _ => VoxelMaterial::new(RenderClass::Opaque, FaceOcclusion::Solid),
        }
    }
}

#[test]
fn an_infinite_solid_world_emits_no_faces() {
    assert_eq!(
        mesh_block_section(&Solid, SectionCoord::new(0, 0, 0)).face_count(),
        0
    );
}

#[test]
fn isolated_solid_section_emits_only_its_exterior() {
    let mesh = mesh_block_section(&OneSection, SectionCoord::new(0, 0, 0));
    assert_eq!(mesh.face_count(), 6 * 16 * 16);
    assert_eq!(mesh.triangle_count(), 12 * 16 * 16);
    assert_eq!(mesh.opaque.vertices.len(), mesh.face_count() * 4);
    assert_eq!(mesh.opaque.indices.len(), mesh.face_count() * 6);
}

#[test]
fn adjacent_sections_do_not_emit_internal_boundary_faces() {
    let left = mesh_block_section(&TwoSections, SectionCoord::new(0, 0, 0));
    let right = mesh_block_section(&TwoSections, SectionCoord::new(1, 0, 0));
    assert_eq!(left.face_count(), 5 * 16 * 16);
    assert_eq!(right.face_count(), 5 * 16 * 16);
    assert!(
        !left
            .opaque
            .vertices
            .iter()
            .any(|vertex| { vertex.position[0] == 16 * 16 && vertex.face == VoxelFace::Right })
    );
    assert!(
        !right
            .opaque
            .vertices
            .iter()
            .any(|vertex| { vertex.position[0] == 0 && vertex.face == VoxelFace::Left })
    );
}

#[test]
fn transparent_material_suppresses_only_matching_internal_faces() {
    let matching = mesh_block_section_with_materials(
        &Pair {
            right: VoxelId::new(4),
        },
        &TestMaterials,
        SectionCoord::new(0, 0, 0),
    );
    assert_eq!(matching.transparent.face_count(), 10);

    let different = mesh_block_section_with_materials(
        &Pair {
            right: VoxelId::new(5),
        },
        &TestMaterials,
        SectionCoord::new(0, 0, 0),
    );
    assert_eq!(different.transparent.face_count(), 12);
}

#[test]
fn geometry_is_split_by_render_class() {
    struct ThreeBlocks;

    impl VoxelSource for ThreeBlocks {
        fn voxel(&self, coord: VoxelCoord) -> VoxelId {
            match (coord.x, coord.y, coord.z) {
                (0, 0, 0) => VoxelId::new(1),
                (2, 0, 0) => VoxelId::new(2),
                (4, 0, 0) => VoxelId::new(3),
                _ => VoxelId::AIR,
            }
        }
    }

    let mesh =
        mesh_block_section_with_materials(&ThreeBlocks, &TestMaterials, SectionCoord::new(0, 0, 0));
    assert_eq!(mesh.opaque.face_count(), 6);
    assert_eq!(mesh.cutout.face_count(), 6);
    assert_eq!(mesh.transparent.face_count(), 6);
}

#[test]
fn section_bounds_are_world_space_for_negative_sections() {
    let bounds = SectionBounds::from_section(SectionCoord::new(-2, 3, -1));
    assert_eq!(bounds.min, VoxelCoord::new(-32, 48, -16));
    assert_eq!(bounds.max_exclusive, VoxelCoord::new(-16, 64, 0));
}

#[test]
fn triangle_winding_matches_each_vertex_normal() {
    let mesh = mesh_block_section(
        &Pair {
            right: VoxelId::AIR,
        },
        SectionCoord::new(0, 0, 0),
    );

    for quad in mesh.opaque.vertices.chunks_exact(4) {
        let edge_a = [
            i32::from(quad[1].position[0]) - i32::from(quad[0].position[0]),
            i32::from(quad[1].position[1]) - i32::from(quad[0].position[1]),
            i32::from(quad[1].position[2]) - i32::from(quad[0].position[2]),
        ];
        let edge_b = [
            i32::from(quad[2].position[0]) - i32::from(quad[0].position[0]),
            i32::from(quad[2].position[1]) - i32::from(quad[0].position[1]),
            i32::from(quad[2].position[2]) - i32::from(quad[0].position[2]),
        ];
        let cross = [
            edge_a[1] * edge_b[2] - edge_a[2] * edge_b[1],
            edge_a[2] * edge_b[0] - edge_a[0] * edge_b[2],
            edge_a[0] * edge_b[1] - edge_a[1] * edge_b[0],
        ];
        assert_eq!(
            cross.map(i32::signum),
            quad[0].normal.map(i32::from),
            "winding differs for {:?}",
            quad[0].face
        );
    }
}

#[test]
fn face_uvs_use_a_top_left_texture_origin() {
    let mesh = mesh_block_section(
        &Pair {
            right: VoxelId::AIR,
        },
        SectionCoord::new(0, 0, 0),
    );
    for quad in mesh.opaque.vertices.chunks_exact(4) {
        assert_eq!(
            quad.iter().map(|vertex| vertex.uv).collect::<Vec<_>>(),
            [[0, 1], [1, 1], [1, 0], [0, 0]]
        );
    }
}

#[test]
fn voxel_corner_ao_samples_neighbours_outside_the_exposed_face() {
    struct Corner;

    impl VoxelSource for Corner {
        fn voxel(&self, coord: VoxelCoord) -> VoxelId {
            if matches!(
                (coord.x, coord.y, coord.z),
                (0, 0, 0) | (-1, 1, 0) | (0, 1, -1)
            ) {
                VoxelId::new(1)
            } else {
                VoxelId::AIR
            }
        }
    }

    let mesh = mesh_block_section(&Corner, SectionCoord::new(0, 0, 0));
    let top = mesh
        .opaque
        .vertices
        .iter()
        .find(|vertex| vertex.face == VoxelFace::Top && vertex.position == [0, 16, 0])
        .expect("target top corner is emitted");
    assert_eq!(top.ambient_occlusion, 0);

    let open = mesh
        .opaque
        .vertices
        .iter()
        .find(|vertex| vertex.face == VoxelFace::Top && vertex.position == [16, 16, 16])
        .expect("opposite top corner is emitted");
    assert_eq!(open.ambient_occlusion, 3);
}

/// A slab (6) with a block (1) above it, and a post (7) beside the slab.
struct Shapes;

impl VoxelSource for Shapes {
    fn voxel(&self, coord: VoxelCoord) -> VoxelId {
        match (coord.x, coord.y, coord.z) {
            (0, 0, 0) => VoxelId::new(6),
            (0, 1, 0) => VoxelId::new(1),
            (1, 0, 0) => VoxelId::new(7),
            _ => VoxelId::AIR,
        }
    }
}

struct ShapedMaterials;

impl VoxelMaterialSource for ShapedMaterials {
    fn material(&self, voxel: VoxelId) -> VoxelMaterial {
        match voxel.value() {
            6 => VoxelMaterial::opaque()
                .with_shape(VoxelShape::from_fractions([0.0; 3], [1.0, 0.5, 1.0])),
            7 => VoxelMaterial::cutout()
                .with_shape(VoxelShape::from_fractions([0.4, 0.0, 0.4], [0.6, 1.0, 0.6])),
            _ => VoxelMaterial::opaque(),
        }
    }
}

fn faces_of(mesh: &crate::BlockMesh, cell: [u8; 3]) -> Vec<(VoxelFace, Vec<[u16; 3]>)> {
    [&mesh.opaque, &mesh.cutout]
        .into_iter()
        .flat_map(|part| part.vertices.chunks_exact(4))
        .filter(|quad| quad[0].cell == cell)
        .map(|quad| {
            (
                quad[0].face,
                quad.iter().map(|vertex| vertex.position).collect(),
            )
        })
        .collect()
}

#[test]
fn a_shape_smaller_than_its_voxel_is_meshed_as_itself() {
    let mesh =
        mesh_block_section_with_materials(&Shapes, &ShapedMaterials, SectionCoord::new(0, 0, 0));
    let slab = faces_of(&mesh, [0, 0, 0]);
    let top = slab
        .iter()
        .find(|(face, _)| *face == VoxelFace::Top)
        .expect("a slab's top is half way up and nothing above meets it");
    assert!(top.1.iter().all(|corner| corner[1] == 8), "{top:?}");
    // The block above a slab shows its underside: the slab does not reach it.
    assert!(
        faces_of(&mesh, [0, 1, 0])
            .iter()
            .any(|(face, _)| *face == VoxelFace::Bottom)
    );
    // A post beside the slab hides none of the slab's side.
    assert!(slab.iter().any(|(face, _)| *face == VoxelFace::Right));
    let post = faces_of(&mesh, [1, 0, 0]);
    assert_eq!(post.len(), 6, "a post has every side");
    let side = post
        .iter()
        .find(|(face, _)| *face == VoxelFace::Right)
        .expect("its east side");
    assert!(side.1.iter().all(|corner| corner[0] == 16 + 10), "{side:?}");
}

#[test]
fn a_voxel_under_a_block_is_marked_covered() {
    let mesh =
        mesh_block_section_with_materials(&Shapes, &ShapedMaterials, SectionCoord::new(0, 0, 0));
    let covered = |cell: [u8; 3]| {
        mesh.opaque
            .vertices
            .iter()
            .chain(&mesh.cutout.vertices)
            .find(|vertex| vertex.cell == cell)
            .map(|vertex| vertex.covered)
    };
    assert_eq!(covered([0, 0, 0]), Some(true), "the slab has a block on it");
    assert_eq!(covered([0, 1, 0]), Some(false), "nothing is on the block");
    assert_eq!(covered([1, 0, 0]), Some(false));
}
