use glam::{Mat4, Vec3};

use super::*;

fn node(position: Vec3, children: Vec<usize>, mesh: Option<usize>) -> ModelHierarchyNode {
    ModelHierarchyNode {
        name: None,
        local_transform: Mat4::from_translation(position),
        children,
        mesh,
    }
}

#[test]
fn hierarchy_retains_nodes_and_composes_local_matrices() {
    let model = RenderModel::new(ModelData {
        nodes: vec![
            node(Vec3::Y, vec![1], None),
            node(Vec3::X * 2.0, vec![], Some(0)),
        ],
        roots: vec![0],
        meshes: vec![vec![]],
        ..ModelData::default()
    })
    .unwrap();
    assert_eq!(model.instances()[0].node, 1);
    assert_eq!(model.instances()[0].mesh, 0);
    assert!(
        model.instances()[0]
            .transform
            .transform_point3(Vec3::ZERO)
            .abs_diff_eq(Vec3::new(2.0, 1.0, 0.0), 1e-6)
    );
    assert_eq!(model.data().nodes[0].children, [1]);
}

#[test]
fn hierarchy_rejects_cycles_multiple_parents_and_overlapping_roots() {
    for (nodes, roots) in [
        (vec![node(Vec3::ZERO, vec![0], None)], vec![]),
        (
            vec![
                node(Vec3::ZERO, vec![1, 1], None),
                node(Vec3::ZERO, vec![], None),
            ],
            vec![0],
        ),
        (
            vec![
                node(Vec3::ZERO, vec![1], None),
                node(Vec3::ZERO, vec![], None),
            ],
            vec![0, 1],
        ),
    ] {
        assert!(
            RenderModel::new(ModelData {
                nodes,
                roots,
                ..ModelData::default()
            })
            .is_err()
        );
    }
}

#[test]
fn resource_rejects_bad_indices_materials_and_images() {
    let mut data = ModelData {
        meshes: vec![vec![ModelGeometry {
            vertices: vec![
                ModelVertex {
                    position: [0.0; 3],
                    normal: [0.0, 1.0, 0.0],
                    uv: [0.0; 2]
                };
                3
            ],
            indices: ModelIndexData::U32(vec![0, 1, 3]),
            material: None,
        }]],
        ..ModelData::default()
    };
    assert!(RenderModel::new(data.clone()).is_err());
    data.meshes[0][0].indices = ModelIndexData::U16(vec![0, 1, 2]);
    assert!(RenderModel::new(data.clone()).is_ok());
    data.meshes[0][0].material = Some(0);
    assert!(RenderModel::new(data.clone()).is_err());
    data.surfaces.push(ModelSurface {
        texture: Some(0),
        ..Default::default()
    });
    assert!(RenderModel::new(data.clone()).is_err());
    data.images.push(ModelImage {
        width: 1,
        height: 1,
        rgba: vec![255; 3],
        wrap_s: wgpu::AddressMode::Repeat,
        wrap_t: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
    });
    assert!(RenderModel::new(data).is_err());
}

#[test]
fn singular_and_nonfinite_transforms_are_diagnosed() {
    assert_eq!(
        validate::transform(Mat4::from_scale(Vec3::new(1.0, 0.0, 1.0))),
        Err(ModelRenderError::InvalidTransform)
    );
    assert_eq!(
        validate::transform(Mat4::from_translation(Vec3::splat(f32::NAN))),
        Err(ModelRenderError::InvalidTransform)
    );
}

/// A figure scaled to nothing is skipped, not an error: a game shrinks one
/// away on purpose, and on a phone's short first frame Causeway's title
/// shrank one below the threshold and stopped the game.
#[test]
fn a_model_scaled_to_nothing_is_skipped_rather_than_refused() {
    for scale in [0.0, 1e-5] {
        assert_eq!(
            validate::drawable(Mat4::from_scale(Vec3::splat(scale))),
            Ok(false)
        );
    }
    assert_eq!(
        validate::drawable(Mat4::from_scale(Vec3::splat(0.01))),
        Ok(true)
    );
    assert_eq!(
        validate::drawable(Mat4::from_translation(Vec3::splat(f32::NAN))),
        Err(ModelRenderError::InvalidTransform),
        "a transform that is not finite is still a bug"
    );
}

#[test]
fn resource_rejects_invalid_emission_but_accepts_hdr() {
    for emissive in [[-0.1, 0.0, 0.0], [f32::NAN; 3], [f32::INFINITY; 3]] {
        assert!(
            RenderModel::new(ModelData {
                surfaces: vec![ModelSurface {
                    emissive,
                    ..Default::default()
                }],
                ..Default::default()
            })
            .is_err()
        );
    }
    assert!(
        RenderModel::new(ModelData {
            surfaces: vec![ModelSurface {
                emissive: [2.0, 0.0, 0.0],
                ..Default::default()
            }],
            ..Default::default()
        })
        .is_ok()
    );
}
