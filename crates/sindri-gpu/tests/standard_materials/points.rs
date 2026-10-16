//! Local lights reach every geometry path, independently of ambient/sun light.
use super::*;
use sindri_render::PointLight;

fn lighting(point: PointLight) -> WorldLighting {
    let mut lighting = WorldLighting {
        ambient_intensity: 0.0,
        ..WorldLighting::default()
    };
    lighting.local_lights[0] = Some(point.into());
    lighting
}

#[test]
fn point_lights_share_shading_range_and_updates_without_geometry_uploads() {
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
    let point = PointLight {
        position: [0.0, 0.0, 2.0],
        color: [0.1, 0.3, 1.0],
        intensity: 8.0,
        range: 6.0,
    };
    for kind in 0..4 {
        let image = scene.draw(&gpu, kind, Some(material), &lighting(point));
        let left = pixel(&image, 32);
        let right = pixel(&image, 95);
        assert!(
            left[2] > left[0] + 10 && right[2] > right[0] + 10,
            "colored point: {left:?}/{right:?}"
        );
        assert!(
            left.iter().zip(right).all(|(a, b)| a.abs_diff(*b) <= 2),
            "shared point response {kind}: {left:?}/{right:?}"
        );
        for disabled in [
            PointLight {
                range: 0.1,
                ..point
            },
            PointLight {
                intensity: 0.0,
                ..point
            },
            PointLight {
                position: [0.0, 0.0, -2.0],
                ..point
            },
            PointLight {
                range: f32::NAN,
                ..point
            },
        ] {
            let image = scene.draw(&gpu, kind, Some(material), &lighting(disabled));
            assert_eq!(&pixel(&image, 32)[..3], &[0; 3]);
            assert_eq!(&pixel(&image, 95)[..3], &[0; 3]);
        }
        let shorter = scene.draw(
            &gpu,
            kind,
            Some(material),
            &lighting(PointLight {
                range: 3.0,
                ..point
            }),
        );
        assert!(
            pixel(&shorter, 32)[2] + 5 < left[2],
            "range changes falloff inside its boundary"
        );
        // Legacy Lambert meshes also receive local lights.
        let image = scene.draw(&gpu, kind, None, &lighting(point));
        assert!(pixel(&image, 32)[2] > pixel(&image, 32)[0] + 10);
        let mut two = lighting(point);
        two.local_lights[7] = Some(
            PointLight {
                color: [1.0, 0.0, 0.0],
                ..point
            }
            .into(),
        );
        let image = scene.draw(&gpu, kind, Some(material), &two);
        assert!(pixel(&image, 32)[0] > left[0] + 10, "second light adds red");
    }
    assert_eq!(scene.cube.model_cache_stats().uploads, 1);
    assert_eq!(scene.cube.cached_mesh_stats().buffer_uploads, 2);
}
