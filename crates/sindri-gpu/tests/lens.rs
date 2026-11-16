//! The composite's lens: where the eye sees in the dark (`night_vision`), light
//! is never blurred, even out of the band in focus, while without it the depth
//! of field softens whatever is out of focus.

use glam::{Mat4, Vec3};
use sindri_gpu::{GpuContext, GpuRequestOptions};
use sindri_render::{
    Bloom, ClearOperations, DepthOfFieldSettings, DepthTarget, NightVisionSettings,
    OffscreenTarget, OrthographicCamera, PostProcessSettings, SpriteBatchDraw, SpriteBatchRenderer,
    SpriteBatchTarget, SpriteDepth, SpriteEnvironment, SpriteInstance, Texture2D, TextureRegistry,
    encode_clear,
};

/// Set wherever a software adapter is installed on purpose; see
/// `sprite_uv_rect.rs`.
const REQUIRE_GPU: &str = "SINDRI_REQUIRE_GPU";

const SIZE: u32 = 256;

fn gpu() -> Option<GpuContext> {
    let instance = wgpu::Instance::default();
    match pollster::block_on(GpuContext::request(
        &instance,
        None,
        &GpuRequestOptions::default(),
    )) {
        Ok(gpu) => Some(gpu),
        Err(error) => {
            assert!(
                std::env::var_os(REQUIRE_GPU).is_none(),
                "{REQUIRE_GPU} is set but no adapter could be requested: {error}"
            );
            eprintln!("skipping: no GPU adapter ({error})");
            None
        }
    }
}

/// A white block over the top left of a black frame, its right edge down the
/// middle, seen through a lens focused low on the screen: the block is out of
/// focus. Answers the red of a pixel just inside its edge.
fn just_inside_the_edge(gpu: &GpuContext, night: bool) -> u8 {
    let target = OffscreenTarget::new(&gpu.device, SIZE, SIZE).expect("the target is valid");
    let depth = DepthTarget::new(&gpu.device, SIZE, SIZE);
    let mut bloom = Bloom::new(&gpu.device, OffscreenTarget::FORMAT);
    bloom.resize(&gpu.device, SIZE, SIZE);
    let mut textures = TextureRegistry::new(&gpu.device, &gpu.queue);
    let white = textures.insert(
        Texture2D::from_rgba8(&gpu.device, &gpu.queue, "white", 1, 1, &[255; 4])
            .expect("one white texel is a texture"),
    );
    let view_projection = OrthographicCamera {
        center: glam::Vec2::ZERO,
        vertical_size: 1.0,
        near: 0.0,
        far: 10.0,
    }
    .view_projection(1.0);
    let block = Mat4::from_translation(Vec3::new(-0.25, 0.25, 0.0))
        * Mat4::from_scale(Vec3::new(0.5, 0.5, 1.0));
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let scene = bloom.scene_view().expect("sized");
    encode_clear(&mut encoder, scene, &depth, ClearOperations::default());
    let mut sprites = SpriteBatchRenderer::new(&gpu.device, bloom.scene_format());
    sprites.begin_submission();
    sprites.set_environment(&SpriteEnvironment {
        time: 0.0,
        lighting: None,
    });
    sprites
        .draw_shaded(
            &gpu.device,
            &gpu.queue,
            &mut encoder,
            SpriteBatchTarget {
                color: scene,
                depth: &depth,
            },
            &textures,
            SpriteBatchDraw {
                texture: white,
                normal_map: None,
                mask: None,
                shader: None,
                view_projection,
                depth: SpriteDepth::Ignore,
                instances: &[SpriteInstance::new(block, [1.0; 4])],
            },
        )
        .expect("the batch fits");
    let settings = PostProcessSettings {
        depth_of_field: DepthOfFieldSettings {
            enabled: true,
            focus: 0.9,
            range: 0.02,
            falloff: 0.1,
            blur: 0.02,
        },
        night_vision: NightVisionSettings {
            enabled: night,
            threshold: 0.14,
            blur: 0.007,
            grey: 0.9,
        },
        ..PostProcessSettings::default()
    };
    bloom.resolve(
        &gpu.device,
        &gpu.queue,
        &mut encoder,
        target.view(),
        settings,
    );
    let readback = target
        .copy_to_buffer(&gpu.device, &mut encoder)
        .expect("the target copies back");
    gpu.queue.submit([encoder.finish()]);
    let pixels = readback
        .read_rgba8(&gpu.device)
        .expect("the frame reads back");
    // Two pixels inside the block's right edge, a quarter of the way down.
    let (x, y) = (SIZE / 2 - 2, SIZE / 4);
    pixels[((y * SIZE + x) * 4) as usize]
}

#[test]
fn light_out_of_focus_stays_sharp_where_the_eye_sees_in_the_dark() {
    let Some(gpu) = gpu() else {
        return;
    };
    let sharp = just_inside_the_edge(&gpu, true);
    let soft = just_inside_the_edge(&gpu, false);
    assert!(sharp >= 250, "lit, it is not blurred: {sharp}");
    assert!(
        soft + 15 < sharp,
        "without night vision the lens softens it: {soft}"
    );
}
