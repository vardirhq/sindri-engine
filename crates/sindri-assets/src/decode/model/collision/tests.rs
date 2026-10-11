use super::*;
use crate::{AssetBytes, AssetDecoder, ModelAssetDecoder};
use sindri_core::AssetId;

fn fixture() -> ModelAsset {
    ModelAssetDecoder
        .decode(AssetBytes::new(
            AssetId::new("models/static.glb").unwrap(),
            include_bytes!("../../../../tests/fixtures/models/static-model.glb").to_vec(),
        ))
        .unwrap()
}

fn near(actual: [f32; 3], expected: [f32; 3]) {
    assert!(
        actual
            .into_iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() < 1.0e-5),
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn glb_hierarchy_expands_instances_in_authored_order_without_mutating_the_asset() {
    let source = fixture();
    let mesh = source
        .collision_mesh(ModelCollisionLimits::default())
        .unwrap();
    assert_eq!(
        mesh.parts.iter().map(|part| part.node).collect::<Vec<_>>(),
        [1, 1, 2, 2, 3]
    );
    assert_eq!(mesh.positions.len(), 15);
    assert_eq!(mesh.triangles.len(), 5);
    assert_eq!(mesh.triangles[0], [0, 1, 2]);
    assert_eq!(mesh.triangles[1], [5, 4, 3]);
    assert_eq!(mesh.parts[4].vertices, 12..15);
    assert_eq!(mesh.parts[4].triangles, 4..5);
    assert_eq!(mesh.parts[4].mesh, 1);
    assert_eq!(mesh.parts[1].primitive, 1);
    near(mesh.positions[0], [1.0, 3.0, 3.0]);
    near(mesh.positions[1], [3.0, 3.0, 3.0]);
    near(mesh.positions[2], [1.0, 6.0, 3.0]);
    assert_eq!(
        mesh,
        source
            .collision_mesh(ModelCollisionLimits::default())
            .unwrap()
    );
    assert_eq!(source.meshes[0].primitives[0].indices.to_u32(), [0, 1, 2]);
}

#[test]
fn reflections_reverse_winding_and_unselected_nodes_stay_out() {
    let mut source = fixture();
    source.roots = vec![2];
    source.nodes[2].children.clear();
    source.nodes[2].local_transform = matrix::IDENTITY;
    source.nodes[2].local_transform[0][0] = -2.0;
    let mesh = source
        .collision_mesh(ModelCollisionLimits::default())
        .unwrap();
    assert_eq!(mesh.parts.len(), 2);
    assert_eq!(mesh.triangles[0], [0, 2, 1]);
    near(mesh.positions[1], [-2.0, 0.0, 0.0]);
    near(mesh.positions[2], [0.0, 1.0, 0.0]);
}

#[test]
fn limits_include_repeated_meshes_and_accept_the_exact_bound() {
    let source = fixture();
    assert!(
        source
            .collision_mesh(ModelCollisionLimits {
                nodes: 4,
                vertices: 15,
                triangles: 5
            })
            .is_ok()
    );
    assert_eq!(
        source.collision_mesh(ModelCollisionLimits {
            nodes: 4,
            vertices: 14,
            triangles: 5
        }),
        Err(ModelCollisionError::VertexLimit)
    );
    assert_eq!(
        source.collision_mesh(ModelCollisionLimits {
            nodes: 4,
            vertices: 15,
            triangles: 4
        }),
        Err(ModelCollisionError::TriangleLimit)
    );
    assert_eq!(
        source.collision_mesh(ModelCollisionLimits {
            nodes: 3,
            ..ModelCollisionLimits::default()
        }),
        Err(ModelCollisionError::NodeLimit)
    );
}

#[test]
fn deep_selected_hierarchies_use_bounded_iterative_traversal() {
    let mut source = fixture();
    let mut node = source.nodes[2].clone();
    node.children.clear();
    node.mesh = None;
    source.nodes = vec![node; 8192];
    for (index, node) in source.nodes.iter_mut().enumerate() {
        if index < 8191 {
            node.children = vec![index + 1];
        } else {
            node.mesh = Some(0);
        }
    }
    source.roots = vec![0];
    assert_eq!(
        source
            .collision_mesh(ModelCollisionLimits::default())
            .unwrap()
            .triangles
            .len(),
        2
    );
    assert_eq!(
        source.collision_mesh(ModelCollisionLimits {
            nodes: 8191,
            ..ModelCollisionLimits::default()
        }),
        Err(ModelCollisionError::NodeLimit)
    );
}

#[test]
fn malformed_mutable_geometry_and_hierarchy_return_typed_errors() {
    let source = fixture();
    for indices in [vec![], vec![0, 1], vec![0, 1, 9], vec![0, 0, 1]] {
        let mut changed = source.clone();
        changed.meshes[0].primitives[0].indices = super::super::ModelIndices::U32(indices);
        assert!(matches!(
            changed.collision_mesh(ModelCollisionLimits::default()),
            Err(ModelCollisionError::Geometry { .. })
        ));
    }
    for roots in [vec![], vec![999], vec![0, 1]] {
        let mut changed = source.clone();
        changed.roots = roots;
        assert!(
            changed
                .collision_mesh(ModelCollisionLimits::default())
                .is_err()
        );
    }
    let mut changed = source.clone();
    changed.nodes[3].children = vec![0];
    assert_eq!(
        changed.collision_mesh(ModelCollisionLimits::default()),
        Err(ModelCollisionError::Hierarchy(0))
    );
    let mut changed = source.clone();
    changed.nodes[1].mesh = Some(999);
    assert!(matches!(
        changed.collision_mesh(ModelCollisionLimits::default()),
        Err(ModelCollisionError::Geometry {
            reason: "missing mesh",
            ..
        })
    ));
}

#[test]
fn singular_projective_nonfinite_and_overflowing_transforms_are_rejected() {
    for value in [0.0, f32::NAN, f32::INFINITY] {
        let mut source = fixture();
        source.nodes[0].local_transform[0][0] = value;
        assert_eq!(
            source.collision_mesh(ModelCollisionLimits::default()),
            Err(ModelCollisionError::Transform(0))
        );
    }
    let mut source = fixture();
    source.nodes[0].local_transform[0][3] = 0.001;
    assert_eq!(
        source.collision_mesh(ModelCollisionLimits::default()),
        Err(ModelCollisionError::Transform(0))
    );
    let mut source = fixture();
    source.nodes[0].local_transform[0][0] = f32::MAX;
    assert_eq!(
        source.collision_mesh(ModelCollisionLimits::default()),
        Err(ModelCollisionError::Transform(1))
    );
    let mut source = fixture();
    source.meshes[0].primitives[0].positions[1][0] = f32::MAX;
    assert!(matches!(
        source.collision_mesh(ModelCollisionLimits::default()),
        Err(ModelCollisionError::Geometry {
            reason: "nonfinite or overflowing vertex",
            ..
        })
    ));
}
