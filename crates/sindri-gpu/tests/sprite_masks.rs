//! Whether a sprite mask decides where a batch draws.
//!
//! The mask rides in the batch uniform and is read in the fragment stage, so
//! nothing short of drawing shows it working: a full-frame white sprite is
//! masked by a quad over the right half, and the two halves are read back.

use glam::{Mat4, Vec3};
use sindri_gpu::{GpuContext, GpuRequestOptions};
use sindri_render::{
    ClearOperations, DepthTarget, OffscreenTarget, OrthographicCamera, SpriteBatchDraw,
    SpriteBatchRenderer, SpriteBatchTarget, SpriteDepth, SpriteEnvironment, SpriteInstance,
    SpriteMask, Texture2D, TextureRegistry, UvRect, encode_clear,
};

/// Set wherever a software adapter is installed on purpose; see
/// `sprite_uv_rect.rs`.
const REQUIRE_GPU: &str = "SINDRI_REQUIRE_GPU";

const SIZE: u32 = 32;

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

/// Draws a full-frame white sprite masked by the right half of the frame,
/// inside or outside, and reads back the middle of each half.
fn draw(gpu: &GpuContext, outside: bool) -> ([u8; 4], [u8; 4]) {
    let target = OffscreenTarget::new(&gpu.device, SIZE, SIZE).expect("the target is valid");
    let depth = DepthTarget::new(&gpu.device, SIZE, SIZE);
    let mut textures = TextureRegistry::new(&gpu.device, &gpu.queue);
    let white = textures.insert(
        Texture2D::from_rgba8(&gpu.device, &gpu.queue, "white", 1, 1, &[255; 4])
            .expect("one white texel is a texture"),
    );
    let mut sprites = SpriteBatchRenderer::new(&gpu.device, OffscreenTarget::FORMAT);
    let view_projection = OrthographicCamera {
        center: glam::Vec2::ZERO,
        vertical_size: 1.0,
        near: 0.0,
        far: 10.0,
    }
    .view_projection(1.0);
    let right_half = Mat4::from_translation(Vec3::new(0.5, 0.0, 0.0))
        * Mat4::from_scale(Vec3::new(1.0, 2.0, 1.0));
    let whole = SpriteInstance::new(Mat4::from_scale(Vec3::new(2.0, 2.0, 1.0)), [1.0; 4]);

    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Sindri sprite mask test encoder"),
        });
    encode_clear(
        &mut encoder,
        target.view(),
        &depth,
        ClearOperations::default(),
    );
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
                color: target.view(),
                depth: &depth,
            },
            &textures,
            SpriteBatchDraw {
                texture: white,
                normal_map: None,
                mask: Some(SpriteMask {
                    texture: white,
                    uv: UvRect::FULL,
                    from_world: right_half.inverse(),
                    outside,
                }),
                shader: None,
                view_projection,
                depth: SpriteDepth::Ignore,
                instances: &[whole],
            },
        )
        .expect("the batch fits");
    let readback = target
        .copy_to_buffer(&gpu.device, &mut encoder)
        .expect("the target copies back");
    gpu.queue.submit([encoder.finish()]);
    let pixels = readback
        .read_rgba8(&gpu.device)
        .expect("the frame reads back");
    let at = |x: u32| {
        let offset = ((SIZE / 2) * SIZE + x) as usize * 4;
        [
            pixels[offset],
            pixels[offset + 1],
            pixels[offset + 2],
            pixels[offset + 3],
        ]
    };
    (at(SIZE / 4), at(SIZE * 3 / 4))
}

fn white(pixel: [u8; 4]) -> bool {
    pixel[..3].iter().all(|channel| *channel > 240)
}

#[test]
fn inside_draws_only_within_the_mask() {
    let Some(gpu) = gpu() else {
        return;
    };
    let (left, right) = draw(&gpu, false);
    assert!(!white(left), "outside the mask drew {left:?}");
    assert!(white(right), "inside the mask drew {right:?}");
}

#[test]
fn outside_draws_only_beyond_the_mask() {
    let Some(gpu) = gpu() else {
        return;
    };
    let (left, right) = draw(&gpu, true);
    assert!(white(left), "beyond the mask drew {left:?}");
    assert!(!white(right), "within the mask drew {right:?}");
}
