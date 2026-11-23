use super::{
    Arc, ClearOperations, DepthTarget, FrameCamera, FrameTarget, GpuContext, Mat4, ModelData,
    ModelImage, OffscreenTarget, RenderModel, TexturedCubeRenderer, Vec3, encode_clear, gpu,
    resource,
};

fn mapped_image(pixel: [u8; 4]) -> ModelImage {
    ModelImage {
        width: 1,
        height: 1,
        rgba: pixel.to_vec(),
        wrap_s: wgpu::AddressMode::ClampToEdge,
        wrap_t: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
    }
}

fn sample(gpu: &GpuContext, data: ModelData, direction: [f32; 3]) -> u8 {
    let asset = Arc::new(RenderModel::new(data).unwrap());
    let target = OffscreenTarget::new(&gpu.device, 64, 64).unwrap();
    let depth = DepthTarget::new(&gpu.device, 64, 64);
    let mut renderer = TexturedCubeRenderer::new(&gpu.device, OffscreenTarget::FORMAT);
    renderer.set_lighting(sindri_render::WorldLighting {
        ambient_intensity: 0.0,
        directional_direction: direction,
        directional_intensity: 2.0,
        ..Default::default()
    });
    renderer.begin_submission();
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encode_clear(
        &mut encoder,
        target.view(),
        &depth,
        ClearOperations::default(),
    );
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
                    position: Vec3::new(0.5, 0.0, 5.0),
                },
            ),
        )
        .unwrap();
    let readback = target.copy_to_buffer(&gpu.device, &mut encoder).unwrap();
    gpu.queue.submit([encoder.finish()]);
    readback.read_rgba8(&gpu.device).unwrap()[(32 * 64 + 48) * 4 + 1]
}

#[test]
fn model_normal_map_follows_uv_orientation() {
    let Some(gpu) = gpu() else { return };
    let mut data = resource().data().clone();
    data.surfaces[1].metallic = 0.0;
    data.surfaces[1].normal_texture = Some(data.images.len());
    data.images.push(mapped_image([255, 128, 128, 255]));
    for vertex in &mut data.meshes[1][0].vertices {
        vertex.uv = [vertex.position[0] + 0.5, 0.5 - vertex.position[1]];
    }
    let from_right = sample(&gpu, data.clone(), [-1.0, 0.0, 0.0]);
    for vertex in &mut data.meshes[1][0].vertices {
        vertex.uv[0] = 1.0 - vertex.uv[0];
    }
    let mirrored = sample(&gpu, data.clone(), [-1.0, 0.0, 0.0]);
    for vertex in &mut data.meshes[1][0].vertices {
        vertex.uv = [0.5, 0.5];
    }
    let degenerate = sample(&gpu, data, [-1.0, 0.0, 0.0]);
    assert!(
        from_right > 150 && mirrored < 5 && degenerate < 5,
        "right: {from_right}, mirrored: {mirrored}, degenerate: {degenerate}"
    );
}

#[test]
fn model_packed_texture_modulates_scalar_factors_in_linear_space() {
    let Some(gpu) = gpu() else { return };
    let mut data = resource().data().clone();
    data.surfaces[1].base_color[1] = 0.3;
    data.surfaces[1].roughness = 0.6;
    data.surfaces[1].metallic = 0.8;
    let direction = [0.2, 0.0, -1.0];
    let original = sample(&gpu, data.clone(), direction);
    let mut mapped = data.clone();
    mapped.surfaces[1].metallic_roughness_texture = Some(mapped.images.len());
    mapped.images.push(mapped_image([255, 160, 120, 255]));
    data.surfaces[1].roughness *= 160.0 / 255.0;
    data.surfaces[1].metallic *= 120.0 / 255.0;
    let scalar = sample(&gpu, data, direction);
    let texture = sample(&gpu, mapped, direction);
    assert!(
        scalar.abs_diff(original) > 5,
        "test needs contrasting material factors: original {original}, scalar {scalar}"
    );
    assert!(
        scalar.abs_diff(texture) < 3,
        "scalar: {scalar}, texture: {texture}"
    );
}
