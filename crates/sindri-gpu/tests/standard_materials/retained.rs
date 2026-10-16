//! Real u32 indices, shadow/color sharing and resource retirement.
use super::*;
use sindri_render::{RenderMesh, ShadowSettings};

#[test]
fn retained_u32_geometry_uploads_once_across_shadow_and_color_then_releases() {
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
    let template = scene.frames[2].clone();
    let FrameCommand::RetainedTexturedMesh {
        geometry, texture, ..
    } = &template.passes()[0].command
    else {
        panic!()
    };
    let mut vertices = vec![TexturedVertex::new([0.0; 3], [0.5; 2]); 65_535];
    vertices.extend_from_slice(geometry.vertices());
    let geometry = Arc::new(
        RenderMesh::new(vertices, vec![65535, 65536, 65537, 65535, 65537, 65538]).unwrap(),
    );
    let mut frame = ExtractedFrame::new(template.viewport(), template.clear());
    for x in [-0.6, 0.6] {
        let mut pass = template.passes()[0].clone();
        pass.command = FrameCommand::RetainedTexturedMesh {
            model: Mat4::from_translation(Vec3::new(x, 0.0, 0.25)),
            texture: *texture,
            geometry: Arc::clone(&geometry),
        };
        frame.push(pass);
    }
    scene.frames[2] = frame.prepare().unwrap();
    let lighting = WorldLighting {
        ambient_intensity: 1.0,
        directional_intensity: 1.0,
        ..Default::default()
    };
    scene.cube.set_shadows(
        &gpu.device,
        ShadowSettings {
            enabled: true,
            map_size: 256,
            ..Default::default()
        },
    );
    let first = scene.draw(&gpu, 2, None, &lighting);
    assert!(
        pixel(&first, 32)[0] > 100 && pixel(&first, 95)[0] > 100,
        "u32 indexed quads draw on both sides"
    );
    assert_eq!(scene.cube.cached_mesh_stats().buffer_uploads, 1);
    assert_eq!(scene.cube.draw_stats().transient_geometry_uploads, 0);
    let shadow_draws = scene.cube.draw_stats().draw_calls;
    assert_eq!(shadow_draws, 4, "two shadow and two color draws");
    assert_eq!(first, scene.draw(&gpu, 2, None, &lighting));
    assert_eq!(scene.cube.cached_mesh_stats().buffer_uploads, 1);
    scene
        .cube
        .set_shadows(&gpu.device, ShadowSettings::default());
    scene.draw(&gpu, 2, None, &lighting);
    assert_eq!(scene.cube.cached_mesh_stats().buffer_uploads, 1);
    scene.frames.clear();
    drop(geometry);
    scene.frames.push(
        ExtractedFrame::new(Viewport::new(128, 128), ClearOperations::default())
            .prepare()
            .unwrap(),
    );
    scene.draw(&gpu, 0, None, &lighting);
    assert_eq!(scene.cube.cached_mesh_stats().entries, 0);
    assert_eq!(scene.cube.cached_mesh_stats().releases, 1);
}

#[test]
fn replaced_resources_keep_old_frames_drawable_and_each_renderer_owns_its_upload() {
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
    let old = scene.frames[2].clone();
    let lighting = WorldLighting::default();
    let original = scene.draw(&gpu, 2, None, &lighting);
    let mut changed = ExtractedFrame::new(old.viewport(), old.clear());
    let mut pass = old.passes()[0].clone();
    let FrameCommand::RetainedTexturedMesh { geometry, .. } = &mut pass.command else {
        panic!()
    };
    let mut vertices = geometry.vertices().to_vec();
    for vertex in &mut vertices {
        vertex.position[0] += 0.5;
    }
    *geometry = Arc::new(RenderMesh::new(vertices, geometry.indices().to_vec()).unwrap());
    changed.push(pass);
    scene.frames[2] = changed.prepare().unwrap();
    assert_ne!(original, scene.draw(&gpu, 2, None, &lighting));
    assert_eq!(scene.cube.cached_mesh_stats().buffer_uploads, 2);
    scene.frames[2] = old;
    assert_eq!(original, scene.draw(&gpu, 2, None, &lighting));
    assert_eq!(scene.cube.cached_mesh_stats().buffer_uploads, 2);
    assert_eq!(
        scene.cube.cached_mesh_stats().entries,
        1,
        "replacement retired"
    );
    let mut second_view = Scene::new(&gpu, [0.0; 3]);
    second_view.frames[2] = scene.frames[2].clone();
    second_view.draw(&gpu, 2, None, &lighting);
    assert_eq!(second_view.cube.cached_mesh_stats().buffer_uploads, 1);
    assert_eq!(scene.cube.cached_mesh_stats().buffer_uploads, 2);
}
