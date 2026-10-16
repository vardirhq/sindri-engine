//! Cone direction, feathering and invalid raw input use the shared local-light path.
use super::*;
use sindri_render::{LocalLight, PointLight, SpotCone};

fn lighting(direction: [f32; 3], outer: f32) -> WorldLighting {
    let mut lighting = WorldLighting {
        ambient_intensity: 0.0,
        ..Default::default()
    };
    lighting.local_lights[0] = Some(LocalLight {
        point: PointLight {
            position: [0.0, 0.0, 2.0],
            color: [0.1, 0.3, 1.0],
            intensity: 8.0,
            range: 6.0,
        },
        spot: Some(SpotCone {
            direction,
            inner_angle: 0.15,
            outer_angle: outer,
        }),
    });
    lighting
}

#[test]
fn cones_aim_feather_and_update_every_geometry_without_uploads() {
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
    let mut scene = Scene::new(&gpu, [0.0; 3]);
    let material = MaterialFactors {
        base_color: [0.5, 0.5, 0.5, 1.0],
        roughness: 0.6,
        ..Default::default()
    };
    for kind in 0..3 {
        let left = scene.draw(
            &gpu,
            kind,
            Some(material),
            &lighting([-0.6, 0.0, -1.75], 0.35),
        );
        assert!(pixel(&left, 32)[2] > 20);
        assert_eq!(&pixel(&left, 95)[..3], &[0; 3]);
        let right = scene.draw(
            &gpu,
            kind,
            Some(material),
            &lighting([0.6, 0.0, -1.75], 0.35),
        );
        assert_eq!(&pixel(&right, 32)[..3], &[0; 3]);
        assert!(pixel(&right, 95)[2] > 20);
        assert!(
            pixel(&left, 32)
                .iter()
                .zip(pixel(&right, 95))
                .all(|(a, b)| a.abs_diff(*b) <= 2)
        );
        let feather = scene.draw(
            &gpu,
            kind,
            Some(material),
            &lighting([-0.6, 0.0, -1.75], 0.9),
        );
        let blue = pixel(&feather, 95)[2];
        assert!(blue > 5 && blue < pixel(&right, 95)[2], "feather: {blue}");
        for invalid in [
            lighting([0.0; 3], 0.35),
            lighting([0.0, 0.0, -1.0], 0.1),
            lighting([f32::NAN, 0.0, -1.0], 0.35),
        ] {
            let image = scene.draw(&gpu, kind, Some(material), &invalid);
            assert_eq!(&pixel(&image, 32)[..3], &[0; 3]);
            assert_eq!(&pixel(&image, 95)[..3], &[0; 3]);
        }
        let legacy = scene.draw(&gpu, kind, None, &lighting([-0.6, 0.0, -1.75], 0.35));
        assert!(pixel(&legacy, 32)[2] > 20);
    }
    assert_eq!(scene.cube.model_cache_stats().uploads, 1);
    assert_eq!(scene.cube.cached_mesh_stats().buffer_uploads, 1);
}
