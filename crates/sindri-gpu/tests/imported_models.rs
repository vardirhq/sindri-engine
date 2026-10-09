//! Read back real GPU pixels: hierarchy, texture color, depth, instances and reuse.

use std::sync::Arc;

use glam::{Mat4, Vec3};
use sindri_gpu::{GpuContext, GpuRequestOptions};
use sindri_render::{
    ClearOperations, DepthTarget, FrameCamera, FrameTarget, ModelData, ModelGeometry,
    ModelHierarchyNode, ModelImage, ModelIndexData, ModelSurface, ModelVertex, OffscreenTarget,
    RenderModel, TexturedCubeRenderer, encode_clear,
};

fn resource() -> Arc<RenderModel> {
    let vertices = [(-0.3, -0.5), (0.3, -0.5), (0.3, 0.5), (-0.3, 0.5)]
        .map(|(x, y)| ModelVertex {
            position: [x, y, 0.0],
            normal: [0.0, 0.0, 1.0],
            uv: [0.5, 0.5],
        })
        .to_vec();
    let primitive = |indices, material| ModelGeometry {
        vertices: vertices.clone(),
        indices,
        material: Some(material),
    };
    Arc::new(
        RenderModel::new(ModelData {
            nodes: vec![
                ModelHierarchyNode {
                    name: Some("root".into()),
                    local_transform: Mat4::from_translation(Vec3::new(-0.5, 0.0, 0.0)),
                    children: vec![1],
                    mesh: Some(0),
                },
                ModelHierarchyNode {
                    name: Some("child".into()),
                    local_transform: Mat4::from_translation(Vec3::X),
                    children: vec![],
                    mesh: Some(1),
                },
            ],
            roots: vec![0],
            meshes: vec![
                vec![primitive(ModelIndexData::U16(vec![0, 1, 2, 0, 2, 3]), 0)],
                vec![primitive(ModelIndexData::U32(vec![0, 1, 2, 0, 2, 3]), 1)],
            ],
            surfaces: vec![
                ModelSurface {
                    texture: Some(0),
                    ..Default::default()
                },
                ModelSurface {
                    base_color: [0.0, 1.0, 0.0, 1.0],
                    ..Default::default()
                },
            ],
            images: vec![ModelImage {
                width: 1,
                height: 1,
                rgba: vec![210, 30, 60, 255],
                wrap_s: wgpu::AddressMode::Repeat,
                wrap_t: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Linear,
            }],
        })
        .unwrap(),
    )
}

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
            eprintln!("skipping: no GPU adapter ({error})");
            None
        }
    }
}

#[test]
fn imported_models_render_hierarchy_colors_depth_and_reuse_uploads() {
    let Some(gpu) = gpu() else {
        return;
    };
    let target = OffscreenTarget::new(&gpu.device, 64, 64).unwrap();
    let depth = DepthTarget::new(&gpu.device, 64, 64);
    let mut renderer = TexturedCubeRenderer::new(&gpu.device, OffscreenTarget::FORMAT);
    let asset = resource();
    let camera = FrameCamera {
        view_projection: Mat4::IDENTITY,
        position: Vec3::Z * 5.0,
    };
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encode_clear(
        &mut encoder,
        target.view(),
        &depth,
        ClearOperations::default(),
    );
    renderer.begin_submission();
    // Smaller instance behind: if uniforms were shared, the large front model
    // would disappear from the sampled pixels. Draw the back instance last to
    // verify depth instead of painter's order.
    for world in [
        Mat4::from_translation(Vec3::Z * 0.2),
        Mat4::from_scale_rotation_translation(
            Vec3::new(-0.5, 0.5, 0.5),
            glam::Quat::IDENTITY,
            Vec3::Z * 0.8,
        ),
    ] {
        renderer
            .encode_model(
                &gpu.device,
                &gpu.queue,
                &mut encoder,
                FrameTarget {
                    color: target.view(),
                    depth: &depth,
                },
                &asset,
                (world, camera),
            )
            .unwrap();
    }
    let readback = target.copy_to_buffer(&gpu.device, &mut encoder).unwrap();
    gpu.queue.submit([encoder.finish()]);
    let pixels = readback.read_rgba8(&gpu.device).unwrap();
    let at = |x: usize, y: usize| &pixels[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4];
    assert!(
        at(16, 32)
            .iter()
            .zip([210, 30, 60, 255])
            .all(|(a, b)| (i32::from(*a) - b).abs() < 3),
        "texture sRGB color: {:?}",
        at(16, 32)
    );
    assert_eq!(
        at(48, 32),
        [0, 255, 0, 255],
        "child local transform and linear material"
    );
    assert_eq!(renderer.model_cache_stats().uploads, 1);
    assert_eq!(renderer.model_cache_stats().draws, 4);
    assert_eq!(renderer.model_cache_stats().resident_models, 1);
    renderer.begin_submission();
    assert_eq!(renderer.model_cache_stats().uploads, 1);
    drop(asset);
    renderer.begin_submission();
    assert_eq!(
        renderer.model_cache_stats().resident_models,
        0,
        "released models free GPU resources"
    );
}

#[test]
fn imported_vertex_normals_face_the_directional_light() {
    let Some(gpu) = gpu() else {
        return;
    };
    let target = OffscreenTarget::new(&gpu.device, 64, 64).unwrap();
    let depth = DepthTarget::new(&gpu.device, 64, 64);
    let mut renderer = TexturedCubeRenderer::new(&gpu.device, OffscreenTarget::FORMAT);
    let mut data = resource().data().clone();
    for surface in &mut data.surfaces {
        surface.metallic = 0.0;
    }
    let asset = Arc::new(RenderModel::new(data).unwrap());
    let mut brightness = Vec::new();
    for direction in [[0.0, 0.0, -1.0], [0.0, 0.0, 1.0]] {
        renderer.set_lighting(sindri_render::WorldLighting {
            ambient_intensity: 0.0,
            directional_direction: direction,
            directional_intensity: 2.0,
            ..Default::default()
        });
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encode_clear(
            &mut encoder,
            target.view(),
            &depth,
            ClearOperations::default(),
        );
        renderer.begin_submission();
        renderer
            .encode_model(
                &gpu.device,
                &gpu.queue,
                &mut encoder,
                FrameTarget {
                    color: target.view(),
                    depth: &depth,
                },
                &asset,
                (
                    Mat4::from_translation(Vec3::Z * 0.2),
                    FrameCamera {
                        view_projection: Mat4::IDENTITY,
                        position: Vec3::Z * 5.0,
                    },
                ),
            )
            .unwrap();
        let readback = target.copy_to_buffer(&gpu.device, &mut encoder).unwrap();
        gpu.queue.submit([encoder.finish()]);
        let pixels = readback.read_rgba8(&gpu.device).unwrap();
        brightness.push(pixels[(32 * 64 + 48) * 4 + 1]);
    }
    assert!(
        brightness[0] > 150,
        "authored normal faces the light: {brightness:?}"
    );
    assert!(
        brightness[1] < 5,
        "light behind surface stays dark: {brightness:?}"
    );
}
