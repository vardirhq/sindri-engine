//! Whether a project's sprite shader is what draws the pixels.
//!
//! Everything before the GPU — that a source is checked, that its parameters
//! ride on the instance and that a shader is part of a batch's key — is tested
//! where it lives. None of that would notice a renderer that built the
//! pipelines and then drew with the built-in ones anyway, so this draws and
//! reads the pixels back.

use glam::{Mat4, Vec3};
use sindri_gpu::{GpuContext, GpuRequestOptions};
use sindri_render::{
    ClearOperations, DepthTarget, OffscreenTarget, OrthographicCamera, SpriteBatchDraw,
    SpriteBatchRenderer, SpriteBatchTarget, SpriteDepth, SpriteEnvironment, SpriteInstance,
    SpriteShader, Texture2D, TextureId, TextureRegistry, encode_clear,
};

/// Set wherever a software adapter is installed on purpose; see
/// `sprite_uv_rect.rs`.
const REQUIRE_GPU: &str = "SINDRI_REQUIRE_GPU";

const SIZE: u32 = 32;
const CHANNEL_TOLERANCE: i32 = 3;

/// Paints the sprite whatever its parameters say, ignoring the texture.
const PARAMS: &str = "\
fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    return vec4<f32>(input.params.rgb, 1.0);
}
";

/// Red once a second has passed.
const CLOCK: &str = "\
fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    return vec4<f32>(min(input.time, 1.0), 0.0, 0.0, 1.0);
}
";

/// Green whatever happens, to tell two shaders apart in one frame.
const GREEN: &str = "\
fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> {
    return vec4<f32>(0.0, 1.0, 0.0, 1.0);
}
";

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

/// A sprite covering one half of the frame: `-1` the left, `1` the right.
fn half(side: f32, params: [f32; 4]) -> SpriteInstance {
    SpriteInstance::new(
        Mat4::from_translation(Vec3::new(side * 0.5, 0.0, 0.0))
            * Mat4::from_scale(Vec3::new(1.0, 2.0, 1.0)),
        [1.0; 4],
    )
    .with_shader_params(params)
}

/// One batch to draw: a shader, or the built-in look, and its sprites.
struct Batch<'a> {
    shader: Option<&'a SpriteShader>,
    instances: Vec<SpriteInstance>,
}

/// Draws the batches in order at `time`, and reads back the middle of the left
/// and right halves of the frame.
fn draw(gpu: &GpuContext, time: f32, batches: &[Batch<'_>]) -> ([u8; 4], [u8; 4]) {
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

    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Sindri sprite shader test encoder"),
        });
    encode_clear(
        &mut encoder,
        target.view(),
        &depth,
        ClearOperations::default(),
    );
    sprites.begin_submission();
    sprites.set_environment(&SpriteEnvironment {
        time,
        lighting: None,
    });
    for batch in batches {
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
                    mask: None,
                    shader: batch.shader,
                    view_projection,
                    depth: SpriteDepth::Ignore,
                    instances: &batch.instances,
                },
            )
            .expect("the batch fits");
    }
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

fn assert_near(actual: [u8; 4], expected: [u8; 4], what: &str) {
    let close = actual
        .iter()
        .zip(expected)
        .all(|(a, e)| (i32::from(*a) - i32::from(e)).abs() <= CHANNEL_TOLERANCE);
    assert!(close, "{what}: drew {actual:?} rather than {expected:?}");
}

fn shader(name: &str, source: &str) -> SpriteShader {
    SpriteShader::new(name, source).expect("the test shader compiles")
}

/// Two sprites in one batch, one shader, each painted by its own parameters:
/// the parameters are per instance, which is what keeps them in one draw.
#[test]
fn each_instance_reads_its_own_parameters() {
    let Some(gpu) = gpu() else {
        return;
    };
    let params = shader("shaders/params.wgsl", PARAMS);
    let (left, right) = draw(
        &gpu,
        0.0,
        &[Batch {
            shader: Some(&params),
            instances: vec![
                half(-1.0, [1.0, 0.0, 0.0, 0.0]),
                half(1.0, [0.0, 0.0, 1.0, 0.0]),
            ],
        }],
    );
    assert_near(left, [255, 0, 0, 255], "the red half");
    assert_near(right, [0, 0, 255, 255], "the blue half");
}

/// The frame's clock reaches the shader.
#[test]
fn a_shader_reads_the_frames_time() {
    let Some(gpu) = gpu() else {
        return;
    };
    let clock = shader("shaders/clock.wgsl", CLOCK);
    let batch = |shader| Batch {
        shader: Some(shader),
        instances: vec![half(-1.0, [0.0; 4])],
    };
    let (at_start, _) = draw(&gpu, 0.0, &[batch(&clock)]);
    let (later, _) = draw(&gpu, 2.0, &[batch(&clock)]);
    assert_near(at_start, [0, 0, 0, 255], "at time zero");
    assert_near(later, [255, 0, 0, 255], "after two seconds");
}

/// Two shaders and the built-in look in one submission each draw their own
/// way, and a batch naming no shader is not drawn through the last one used.
#[test]
fn batches_keep_their_own_shaders() {
    let Some(gpu) = gpu() else {
        return;
    };
    let params = shader("shaders/params.wgsl", PARAMS);
    let green = shader("shaders/green.wgsl", GREEN);
    let (left, right) = draw(
        &gpu,
        0.0,
        &[
            Batch {
                shader: Some(&params),
                instances: vec![half(-1.0, [1.0, 0.0, 0.0, 0.0])],
            },
            Batch {
                shader: Some(&green),
                instances: vec![half(1.0, [0.0; 4])],
            },
        ],
    );
    assert_near(left, [255, 0, 0, 255], "the shader on the left");
    assert_near(right, [0, 255, 0, 255], "the shader on the right");

    let (built_in, _) = draw(
        &gpu,
        0.0,
        &[
            Batch {
                shader: Some(&green),
                instances: vec![half(1.0, [0.0; 4])],
            },
            Batch {
                shader: None,
                instances: vec![half(-1.0, [0.0; 4])],
            },
        ],
    );
    assert_near(built_in, [255, 255, 255, 255], "the built-in look");
}

/// A reload of the same file is a new shader, and the renderer draws with it
/// rather than with the pipelines it built for the old one.
#[test]
fn a_reloaded_shader_replaces_the_old_one() {
    let Some(gpu) = gpu() else {
        return;
    };
    let target = OffscreenTarget::new(&gpu.device, SIZE, SIZE).expect("the target is valid");
    let depth = DepthTarget::new(&gpu.device, SIZE, SIZE);
    let textures = TextureRegistry::new(&gpu.device, &gpu.queue);
    let mut sprites = SpriteBatchRenderer::new(&gpu.device, OffscreenTarget::FORMAT);
    for source in [PARAMS, GREEN, PARAMS] {
        let reloaded = shader("shaders/edited.wgsl", source);
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        sprites.begin_submission();
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
                    texture: TextureId::new(0),
                    normal_map: None,
                    mask: None,
                    shader: Some(&reloaded),
                    view_projection: Mat4::IDENTITY,
                    depth: SpriteDepth::Ignore,
                    instances: &[half(-1.0, [0.0; 4])],
                },
            )
            .expect("the batch fits");
        gpu.queue.submit([encoder.finish()]);
    }
    assert_eq!(sprites.shader_count(), 1, "a reload added a shader");
}
