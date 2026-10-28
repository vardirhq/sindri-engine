//! Whether the frame's 2D lights reach the pixels: ambient where nothing
//! reaches, a light's color near it, nothing past a wall that casts shadows,
//! a normal map turning a surface toward or away from a light, and nothing at
//! all on a sprite the extractor left unlit.

use std::sync::Arc;

use glam::{Mat4, Vec3};
use sindri_gpu::{GpuContext, GpuRequestOptions};
use sindri_render::{
    ClearOperations, DepthTarget, Light2d, Light2dShadow, Lighting2d, OffscreenTarget,
    OrthographicCamera, ShapeBlend, ShapeInstance, ShapeRenderer, SpriteBatchDraw,
    SpriteBatchRenderer, SpriteBatchTarget, SpriteDepth, SpriteEnvironment, SpriteInstance,
    Texture2D, TextureRegistry, encode_clear, shadow_map_2d, shadow_maps_2d,
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

/// A red light over the left half's middle of a frame one unit wide,
/// reaching just short of the right half's middle.
fn red_light(shadow: Option<Light2dShadow>) -> Light2d {
    Light2d {
        position: [-0.25, 0.0],
        height: 0.1,
        radius: 0.45,
        falloff: 1.0,
        color: [1.0, 0.0, 0.0],
        shadow,
    }
}

fn dim(lights: Vec<Light2d>) -> Lighting2d {
    Lighting2d {
        ambient: [0.2, 0.2, 0.2],
        lights,
    }
}

/// What to draw over the whole frame.
enum Subject {
    Sprite { lit: bool, normal: Option<[u8; 4]> },
    Shape,
}

/// Draws `subject` covering the frame under `lighting`, and reads back the
/// middle of the left and right halves.
fn draw(gpu: &GpuContext, lighting: Option<Lighting2d>, subject: &Subject) -> ([u8; 4], [u8; 4]) {
    let target = OffscreenTarget::new(&gpu.device, SIZE, SIZE).expect("the target is valid");
    let depth = DepthTarget::new(&gpu.device, SIZE, SIZE);
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
    let model = Mat4::from_scale(Vec3::new(2.0, 2.0, 1.0));
    let environment = SpriteEnvironment {
        time: 0.0,
        lighting,
    };
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    encode_clear(
        &mut encoder,
        target.view(),
        &depth,
        ClearOperations::default(),
    );
    let mut sprites = SpriteBatchRenderer::new(&gpu.device, OffscreenTarget::FORMAT);
    let mut shapes = ShapeRenderer::new(&gpu.device, OffscreenTarget::FORMAT);
    sprites.begin_submission();
    shapes.begin_submission();
    sprites.set_environment(&environment);
    shapes.set_environment(&environment);
    match subject {
        Subject::Sprite { lit, normal } => {
            let normal_map = normal.map(|texel| {
                textures.insert(
                    Texture2D::from_rgba8(&gpu.device, &gpu.queue, "normal", 1, 1, &texel)
                        .expect("one texel is a texture"),
                )
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
                        normal_map,
                        mask: None,
                        shader: None,
                        view_projection,
                        depth: SpriteDepth::Ignore,
                        instances: &[SpriteInstance::new(model, [1.0; 4]).lit(*lit)],
                    },
                )
                .expect("the batch fits");
        }
        Subject::Shape => shapes
            .draw(
                &gpu.device,
                &gpu.queue,
                &mut encoder,
                target.view(),
                &depth,
                ShapeBlend::Over,
                view_projection,
                &[ShapeInstance::filled(model, sindri_render::Shape::Rect, [1.0; 4]).lit(true)],
            )
            .expect("the shapes fit"),
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

const LIT: Subject = Subject::Sprite {
    lit: true,
    normal: None,
};

#[test]
fn a_lit_sprite_takes_the_lights_color_and_the_ambient_elsewhere() {
    let Some(gpu) = gpu() else {
        return;
    };
    let (near, far) = draw(&gpu, Some(dim(vec![red_light(None)])), &LIT);
    assert!(
        near[0] > 240 && near[1] < 140,
        "red near the light: {near:?}"
    );
    assert_eq!(far[0], far[1], "only ambient far away: {far:?}");
    assert!(far[0] > 60 && far[0] < 160, "ambient, not white: {far:?}");
}

#[test]
fn an_unlit_sprite_and_a_scene_without_lighting_keep_their_color() {
    let Some(gpu) = gpu() else {
        return;
    };
    let unlit = Subject::Sprite {
        lit: false,
        normal: None,
    };
    let (left, right) = draw(&gpu, Some(dim(vec![red_light(None)])), &unlit);
    assert_eq!((left, right), ([255; 4], [255; 4]));
    let (left, _) = draw(&gpu, None, &LIT);
    assert_eq!(left, [255; 4]);
}

#[test]
fn a_wall_casts_a_shadow() {
    let Some(gpu) = gpu() else {
        return;
    };
    // Reaching the right half's middle, half a unit away, with a wall between.
    let reaching = Light2d {
        radius: 0.6,
        ..red_light(None)
    };
    let wall = [[0.0, -1.0], [0.0, 1.0]];
    let walled = Light2d {
        shadow: Some(Light2dShadow {
            reach: 0.0,
            source_radius: 0.0,
            map: Arc::from(shadow_map_2d([-0.25, 0.0], 0.6, &[wall])),
        }),
        ..reaching.clone()
    };
    let (_, open) = draw(&gpu, Some(dim(vec![reaching])), &LIT);
    let (near, behind) = draw(&gpu, Some(dim(vec![walled])), &LIT);
    assert!(open[0] > open[1] + 20, "lit without the wall: {open:?}");
    assert!(near[0] > 240, "the light's own side is lit: {near:?}");
    assert_eq!(
        behind[0], behind[1],
        "only ambient behind the wall: {behind:?}"
    );
}

#[test]
fn an_area_light_partly_lights_a_shadow_edge() {
    let Some(gpu) = gpu() else {
        return;
    };
    let origin = [-0.25, 0.0];
    let wall = [[[0.0, -1.0], [0.0, 0.0]]];
    let open_light = Light2d {
        radius: 0.8,
        ..red_light(None)
    };
    let area = Light2d {
        shadow: Some(Light2dShadow {
            reach: 0.0,
            source_radius: 0.2,
            map: Arc::from(shadow_maps_2d(origin, 0.8, 0.2, &wall)),
        }),
        ..open_light.clone()
    };
    let (_, open) = draw(&gpu, Some(dim(vec![open_light])), &LIT);
    let (_, partial) = draw(&gpu, Some(dim(vec![area])), &LIT);
    assert!(
        u16::from(partial[0]) > u16::from(partial[1]) + 2,
        "some light reaches the edge: {partial:?}"
    );
    assert!(
        u16::from(partial[0]) + 2 < u16::from(open[0]),
        "the wall blocks part of the light: {partial:?} vs {open:?}"
    );
}

/// The same in the up-and-down direction: a light above a ledge shades what is
/// beneath the ledge, not what is above it.
#[test]
fn a_ledge_shades_what_is_beneath_it() {
    let Some(gpu) = gpu() else {
        return;
    };
    let ledge = [[-1.0, 0.15], [-0.05, 0.15]];
    let above = Light2d {
        position: [-0.25, 0.3],
        radius: 1.0,
        shadow: Some(Light2dShadow {
            reach: 0.0,
            source_radius: 0.0,
            map: Arc::from(shadow_map_2d([-0.25, 0.3], 1.0, &[ledge])),
        }),
        ..red_light(None)
    };
    let (beneath, beside) = draw(&gpu, Some(dim(vec![above])), &LIT);
    assert_eq!(
        beneath[0], beneath[1],
        "only ambient beneath the ledge: {beneath:?}"
    );
    assert!(beside[0] > beside[1] + 20, "lit past its end: {beside:?}");
}

#[test]
fn a_normal_map_turns_a_surface_toward_or_away_from_the_light() {
    let Some(gpu) = gpu() else {
        return;
    };
    // Right of the left sample, so a surface facing right faces the light.
    let lighting = Some(dim(vec![Light2d {
        position: [0.0, 0.0],
        ..red_light(None)
    }]));
    let facing = |x: u8| Subject::Sprite {
        lit: true,
        normal: Some([x, 128, 255, 255]),
    };
    let (toward, _) = draw(&gpu, lighting.clone(), &facing(240));
    let (away, _) = draw(&gpu, lighting.clone(), &facing(16));
    let (flat, _) = draw(&gpu, lighting, &LIT);
    assert!(
        toward[0] > away[0] + 40,
        "facing the light is brighter: {toward:?} vs {away:?}"
    );
    assert!(
        flat[0] >= toward[0],
        "no map lights by distance alone: {flat:?}"
    );
}

#[test]
fn shapes_are_lit_too() {
    let Some(gpu) = gpu() else {
        return;
    };
    let (near, far) = draw(&gpu, Some(dim(vec![red_light(None)])), &Subject::Shape);
    assert!(
        near[0] > 240 && near[1] < 140,
        "red near the light: {near:?}"
    );
    assert_eq!(far[0], far[1], "ambient far away: {far:?}");
}
