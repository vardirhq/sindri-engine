//! Textured mesh normals match the imported-model inverse-transpose path.
use super::*;

pub(super) fn authored(scene: &mut Scene, normals: [[f32; 3]; 4], transform: Mat4) {
    let FrameCommand::Model { asset, .. } = &scene.frames[1].passes()[1].command else {
        panic!("model expected");
    };
    let mut data = asset.data().clone();
    for (vertex, normal) in data.meshes[0][0].vertices.iter_mut().zip(normals) {
        vertex.normal = normal;
    }
    let asset = Arc::new(RenderModel::new(data).unwrap());
    for prepared in scene.frames.iter_mut().skip(1) {
        let mut frame = ExtractedFrame::new(prepared.viewport(), prepared.clear());
        for mut pass in prepared.passes().iter().cloned() {
            pass.camera.position = Vec3::Z * 10_000.0;
            match &mut pass.command {
                FrameCommand::TexturedMesh {
                    vertices, model, ..
                } => {
                    for (vertex, normal) in vertices.iter_mut().zip(normals) {
                        vertex.normal = normal;
                    }
                    *model = Mat4::from_translation(Vec3::new(-0.6, 0.0, 0.25)) * transform;
                }
                FrameCommand::RetainedTexturedMesh {
                    geometry, model, ..
                } => {
                    let mut vertices = geometry.vertices().to_vec();
                    for (vertex, normal) in vertices.iter_mut().zip(normals) {
                        vertex.normal = normal;
                    }
                    *geometry = Arc::new(
                        sindri_render::RenderMesh::new(vertices, geometry.indices().to_vec())
                            .unwrap(),
                    );
                    *model = Mat4::from_translation(Vec3::new(-0.6, 0.0, 0.25)) * transform;
                }
                FrameCommand::CachedTexturedMesh {
                    replacement, model, ..
                } => {
                    for (vertex, normal) in replacement
                        .as_mut()
                        .unwrap()
                        .vertices
                        .iter_mut()
                        .zip(normals)
                    {
                        vertex.normal = normal;
                    }
                    *model = Mat4::from_translation(Vec3::new(-0.6, 0.0, 0.25)) * transform;
                }
                FrameCommand::Model {
                    model,
                    asset: bound,
                } => {
                    *bound = Arc::clone(&asset);
                    *model = Mat4::from_translation(Vec3::new(0.6, 0.0, 0.25)) * transform;
                }
                _ => panic!("unexpected swatch"),
            }
            frame.push(pass);
        }
        *prepared = frame.prepare().unwrap();
    }
}

#[test]
fn authored_normals_match_models_under_nonuniform_rotation_and_reflection() {
    let gpu = match pollster::block_on(GpuContext::request(
        &wgpu::Instance::default(),
        None,
        &GpuRequestOptions::default(),
    )) {
        Ok(gpu) => gpu,
        Err(error) => {
            assert!(
                std::env::var_os("SINDRI_REQUIRE_GPU").is_none(),
                "required GPU unavailable: {error}"
            );
            return;
        }
    };
    let lighting = WorldLighting {
        ambient_intensity: 0.0,
        directional_intensity: 2.0,
        directional_direction: [-0.4, 0.0, -1.0],
        ..Default::default()
    };
    let material = MaterialFactors {
        base_color: [0.5, 0.3, 0.1, 1.0],
        ..Default::default()
    };
    for scale in [
        Vec3::ONE,
        Vec3::new(0.7, 1.3, 2.0),
        Vec3::new(-0.7, 1.3, 2.0),
    ] {
        let transform = Mat4::from_scale_rotation_translation(
            scale,
            glam::Quat::from_rotation_y(0.35),
            Vec3::ZERO,
        );
        let mut scene = Scene::new(&gpu, [0.0; 3]);
        // Different normals per vertex exercise interpolation as well as upload.
        authored(
            &mut scene,
            [
                [0.7, 0.0, 1.0],
                [0.9, 0.0, 1.0],
                [0.9, 0.3, 1.0],
                [0.7, 0.3, 1.0],
            ],
            transform,
        );
        for kind in [1, 2, 3] {
            let image = scene.draw(&gpu, kind, Some(material), &lighting);
            assert!(
                pixel(&image, 32)
                    .iter()
                    .zip(pixel(&image, 95))
                    .all(|(a, b)| a.abs_diff(*b) <= 2),
                "geometry {kind}, scale {scale:?}: {:?}/{:?}",
                pixel(&image, 32),
                pixel(&image, 95)
            );
        }
        assert_eq!(scene.cube.model_cache_stats().uploads, 1);
        assert_eq!(scene.cube.cached_mesh_stats().buffer_uploads, 2);
    }
}
