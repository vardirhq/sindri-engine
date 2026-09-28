//! Does a sprite draw with the texture its registry holds, after the renderer
//! has drawn from another registry?
//!
//! It did not. The renderer kept one bind group per texture handle, and handles
//! are numbered from one in every registry, so the first texture of the scene
//! opened next shared a handle — and so a bind group — with the first texture
//! of the scene before it. The editor makes a registry per scene and keeps one
//! renderer, and after closing Orbital and opening the platformer it drew the
//! platformer's ground with Orbital's ship.

use glam::{Mat4, Vec3};
use sindri_gpu::{GpuContext, GpuRequestOptions};
use sindri_render::{
    ClearOperations, DepthTarget, OffscreenTarget, OrthographicCamera, SpriteBatchRenderer,
    SpriteDepth, SpriteInstance, Texture2D, TextureId, TextureRegistry, encode_clear,
};

const REQUIRE_GPU: &str = "SINDRI_REQUIRE_GPU";
const SIZE: u32 = 32;
const CHANNEL_TOLERANCE: i32 = 4;

const FIRST: [u8; 4] = [220, 60, 60, 255];
const SECOND: [u8; 4] = [60, 120, 240, 255];

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

/// A registry holding one solid texture, and its handle.
fn registry_of(gpu: &GpuContext, color: [u8; 4]) -> (TextureRegistry, TextureId) {
    let mut registry = TextureRegistry::new(&gpu.device, &gpu.queue);
    let texture = Texture2D::from_rgba8(&gpu.device, &gpu.queue, "solid", 1, 1, &color)
        .expect("a one-pixel texture is valid");
    let id = registry.insert(texture);
    (registry, id)
}

/// Draws one full-frame sprite and returns the colour in the middle.
fn draw(
    gpu: &GpuContext,
    sprites: &mut SpriteBatchRenderer,
    registry: &TextureRegistry,
    texture: TextureId,
) -> [u8; 4] {
    let target = OffscreenTarget::new(&gpu.device, SIZE, SIZE).expect("the target is valid");
    let depth = DepthTarget::new(&gpu.device, SIZE, SIZE);
    let view = OrthographicCamera {
        center: glam::Vec2::ZERO,
        vertical_size: 2.0,
        near: 0.0,
        far: 10.0,
    }
    .view_projection(1.0);
    let quad = SpriteInstance::new(
        Mat4::from_scale(Vec3::new(4.0, 4.0, 1.0)),
        [1.0, 1.0, 1.0, 1.0],
    );
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Sindri registry switch encoder"),
        });
    encode_clear(
        &mut encoder,
        target.view(),
        &depth,
        ClearOperations::default(),
    );
    sprites.begin_submission();
    sprites
        .draw(
            &gpu.device,
            &gpu.queue,
            &mut encoder,
            target.view(),
            &depth,
            registry,
            texture,
            view,
            SpriteDepth::Ignore,
            &[quad],
        )
        .expect("one sprite fits the batch");
    let readback = target
        .copy_to_buffer(&gpu.device, &mut encoder)
        .expect("the target copies back");
    gpu.queue.submit([encoder.finish()]);
    let pixels = readback
        .read_rgba8(&gpu.device)
        .expect("the frame reads back");
    let i = ((SIZE / 2) * SIZE + SIZE / 2) as usize * 4;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

fn near(actual: [u8; 4], expected: [u8; 4]) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(a, e)| (i32::from(*a) - i32::from(e)).abs() <= CHANNEL_TOLERANCE)
}

#[test]
fn a_new_registry_s_texture_is_drawn_not_the_old_one_s() {
    let Some(gpu) = gpu() else {
        return;
    };
    let mut sprites = SpriteBatchRenderer::new(&gpu.device, OffscreenTarget::FORMAT);

    let (first, first_id) = registry_of(&gpu, FIRST);
    let drawn = draw(&gpu, &mut sprites, &first, first_id);
    assert!(
        near(drawn, FIRST),
        "the first scene draws its own texture: {drawn:?}"
    );
    drop(first);

    let (second, second_id) = registry_of(&gpu, SECOND);
    assert_eq!(
        first_id, second_id,
        "the two registries hand out the same handle, which is the whole problem"
    );
    let drawn = draw(&gpu, &mut sprites, &second, second_id);
    assert!(
        near(drawn, SECOND),
        "the second scene drew with the first scene's texture: {drawn:?}"
    );
}
