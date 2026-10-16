//! Back-face visibility, lighting and restoration share one per-draw policy.
use super::*;
use sindri_render::{MaterialSidedness, PreparedFrame};

fn gpu() -> Option<GpuContext> {
    match pollster::block_on(GpuContext::request(
        &wgpu::Instance::default(),
        None,
        &GpuRequestOptions::default(),
    )) {
        Ok(gpu) => Some(gpu),
        Err(error) => {
            assert!(
                std::env::var_os("SINDRI_REQUIRE_GPU").is_none(),
                "required GPU unavailable: {error}"
            );
            None
        }
    }
}

fn sidedness(frame: &PreparedFrame, value: MaterialSidedness) -> PreparedFrame {
    let mut next = ExtractedFrame::new(frame.viewport(), frame.clear());
    for mut pass in frame.passes().iter().cloned() {
        pass.sidedness = value;
        next.push(pass);
    }
    next.prepare().unwrap()
}

#[test]
fn mesh_backfaces_match_model_lighting_and_policy_changes_keep_geometry_resident() {
    let Some(gpu) = gpu() else { return };
    let lighting = WorldLighting {
        ambient_intensity: 0.0,
        directional_intensity: 2.0,
        directional_direction: [-0.4, 0.0, -1.0],
        ..Default::default()
    };
    for x in [0.7, -0.7] {
        let mut scene = Scene::new(&gpu, [0.0; 3]);
        normals::authored(
            &mut scene,
            [[0.7, 0.0, 1.0]; 4],
            Mat4::from_scale_rotation_translation(
                Vec3::new(x, 1.3, 2.0),
                glam::Quat::from_rotation_y(std::f32::consts::PI + 0.35),
                Vec3::ZERO,
            ),
        );
        let material = Some(MaterialFactors {
            base_color: [0.5, 0.3, 0.1, 1.0],
            ..Default::default()
        });
        for kind in 1..4 {
            for value in [
                MaterialSidedness::Inherit,
                MaterialSidedness::Double,
                MaterialSidedness::Single,
                MaterialSidedness::Double,
            ] {
                scene.frames[kind] = sidedness(&scene.frames[kind], value);
                let image = scene.draw(&gpu, kind, material, &lighting);
                let left = pixel(&image, 32);
                let right = pixel(&image, 95);
                if value == MaterialSidedness::Double {
                    assert!(left[0] > 40, "back face receives direct light: {left:?}");
                    assert!(
                        left.iter().zip(right).all(|(a, b)| a.abs_diff(*b) <= 2),
                        "kind {kind}, scale {x}: {left:?}/{right:?}"
                    );
                } else {
                    assert_eq!(left, pixel(&image, 0));
                    assert_eq!(right, pixel(&image, 0));
                }
            }
        }
        assert_eq!(scene.cube.model_cache_stats().uploads, 1);
        assert_eq!(scene.cube.cached_mesh_stats().buffer_uploads, 2);
    }
}

#[test]
fn inherited_model_sidedness_survives_numeric_overrides_and_forced_single_restores() {
    let Some(gpu) = gpu() else { return };
    let mut scene = Scene::new(&gpu, [0.0; 3]);
    normals::authored(
        &mut scene,
        [[0.0, 0.0, 1.0]; 4],
        Mat4::from_rotation_y(std::f32::consts::PI),
    );
    let mut frame = ExtractedFrame::new(scene.frames[2].viewport(), scene.frames[2].clear());
    for mut pass in scene.frames[2].passes().iter().cloned() {
        if let FrameCommand::Model { asset, .. } = &mut pass.command {
            let mut data = asset.data().clone();
            data.surfaces[0].double_sided = true;
            *asset = Arc::new(RenderModel::new(data).unwrap());
        }
        frame.push(pass);
    }
    scene.frames[2] = frame.prepare().unwrap();
    let lighting = WorldLighting {
        ambient_intensity: 1.0,
        directional_intensity: 0.0,
        ..Default::default()
    };
    for value in [
        MaterialSidedness::Inherit,
        MaterialSidedness::Single,
        MaterialSidedness::Double,
        MaterialSidedness::Inherit,
    ] {
        scene.frames[2] = sidedness(&scene.frames[2], value);
        let image = scene.draw(&gpu, 2, Some(MaterialFactors::default()), &lighting);
        assert_eq!(
            pixel(&image, 95)[0] > 100,
            value != MaterialSidedness::Single
        );
        assert_eq!(
            pixel(&image, 32)[0] > 100,
            value == MaterialSidedness::Double
        );
    }
    assert_eq!(scene.cube.model_cache_stats().uploads, 1);
    assert_eq!(scene.cube.cached_mesh_stats().buffer_uploads, 1);
}

#[test]
fn a_cube_seen_from_inside_lights_its_double_sided_backfaces() {
    let Some(gpu) = gpu() else { return };
    let mut scene = Scene::new(&gpu, [0.0; 3]);
    let mut frame = ExtractedFrame::new(scene.frames[0].viewport(), scene.frames[0].clear());
    let mut pass = scene.frames[0].passes()[0].clone();
    let FrameCommand::TexturedCube { model, .. } = &mut pass.command else {
        panic!()
    };
    *model = Mat4::from_scale_rotation_translation(
        Vec3::splat(4.0),
        glam::Quat::IDENTITY,
        Vec3::Z * 5.0,
    );
    frame.push(pass);
    scene.frames[0] = frame.prepare().unwrap();
    let lighting = WorldLighting {
        ambient_intensity: 0.0,
        directional_intensity: 2.0,
        directional_direction: [0.0, 0.0, -1.0],
        ..Default::default()
    };
    for value in [
        MaterialSidedness::Single,
        MaterialSidedness::Double,
        MaterialSidedness::Inherit,
    ] {
        scene.frames[0] = sidedness(&scene.frames[0], value);
        let image = scene.draw(&gpu, 0, Some(MaterialFactors::default()), &lighting);
        assert_eq!(
            pixel(&image, 32)[0] > 100,
            value == MaterialSidedness::Double
        );
    }
    assert_eq!(scene.cube.draw_stats().transient_geometry_uploads, 0);
}

#[test]
fn adjacent_draws_reset_face_visibility_independently() {
    let Some(gpu) = gpu() else { return };
    let mut scene = Scene::new(&gpu, [0.0; 3]);
    normals::authored(
        &mut scene,
        [[0.0, 0.0, 1.0]; 4],
        Mat4::from_rotation_y(std::f32::consts::PI),
    );
    let original = scene.frames[2].clone();
    let lighting = WorldLighting {
        ambient_intensity: 1.0,
        directional_intensity: 0.0,
        ..Default::default()
    };
    for policies in [
        [MaterialSidedness::Double, MaterialSidedness::Single],
        [MaterialSidedness::Single, MaterialSidedness::Double],
        [MaterialSidedness::Inherit, MaterialSidedness::Single],
        [MaterialSidedness::Double, MaterialSidedness::Inherit],
    ] {
        let mut frame = ExtractedFrame::new(original.viewport(), original.clear());
        for (mut pass, policy) in original.passes().iter().cloned().zip(policies) {
            pass.sidedness = policy;
            frame.push(pass);
        }
        scene.frames[2] = frame.prepare().unwrap();
        let image = scene.draw(&gpu, 2, Some(MaterialFactors::default()), &lighting);
        for (x, policy) in [32, 95].into_iter().zip(policies) {
            assert_eq!(
                pixel(&image, x)[0] > 100,
                policy == MaterialSidedness::Double
            );
        }
    }
    assert_eq!(scene.cube.model_cache_stats().uploads, 1);
    assert_eq!(scene.cube.cached_mesh_stats().buffer_uploads, 1);
}
