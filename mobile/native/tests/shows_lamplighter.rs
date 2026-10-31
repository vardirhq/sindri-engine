//! The platform-neutral half of what the Android app shows: Lamplighter's
//! field opened as an authored world, and drawn by the engine.
//!
//! The draw is offscreen, at the Pixel 8's portrait resolution, through the
//! same `SceneRenderer` the app hands its surface to. It proves the frame is
//! built and encoded; that it reaches a phone's screen is the device check in
//! `mobile/first-slice.md`.

use std::path::{Path, PathBuf};

use sindri_core::SceneEntityId;
use sindri_gpu::{GpuContext, GpuRequestOptions};
use sindri_mobile::{OpenedScene, SceneRenderer};
use sindri_render::OffscreenTarget;

/// Set wherever a software adapter is installed on purpose, as in
/// `sindri-gpu`'s tests: a GPU test that skips where it is meant to run has
/// quietly stopped checking.
const REQUIRE_GPU: &str = "SINDRI_REQUIRE_GPU";
/// The Pixel 8's screen, held upright.
const PIXEL_8: [u32; 2] = [1080, 2400];

fn lamplighter() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../games/lamplighter")
        .canonicalize()
        .expect("Lamplighter is in the repository")
}

#[test]
fn the_field_opens_as_the_authored_scene() {
    let opened = OpenedScene::open(&lamplighter(), "world.scene").expect("the scene opens");
    let wick = SceneEntityId::new("wick".to_owned()).expect("a valid ID");
    assert!(opened.world.entity_for_source_id(&wick).is_some());
    assert!(
        opened.world.entities().count() > 100,
        "{}",
        opened.summary()
    );
    assert!(
        !opened.assets.textures.is_empty(),
        "the project's textures are read for drawing"
    );
    assert!(opened.summary().starts_with("world.scene · "));
}

#[test]
fn the_entry_scene_opens_by_its_file_name_and_a_missing_one_is_named() {
    let title = OpenedScene::open(&lamplighter(), "title.scene").expect("the title opens");
    assert_eq!(title.name, "title.scene");
    let error = OpenedScene::open(&lamplighter(), "nowhere.scene")
        .err()
        .expect("a scene the project does not list is refused");
    assert!(error.contains("nowhere.scene"), "{error}");
}

#[test]
fn the_field_draws_at_phone_resolution() {
    let instance = wgpu::Instance::default();
    let gpu = match pollster::block_on(GpuContext::request(
        &instance,
        None,
        &GpuRequestOptions::default(),
    )) {
        Ok(gpu) => gpu,
        Err(error) => {
            assert!(
                std::env::var_os(REQUIRE_GPU).is_none(),
                "{REQUIRE_GPU} is set but no adapter could be requested: {error}"
            );
            eprintln!("skipping: no GPU adapter ({error})");
            return;
        }
    };
    let [width, height] = PIXEL_8;
    let mut opened = OpenedScene::open(&lamplighter(), "world.scene").expect("the scene opens");
    let mut renderer = SceneRenderer::new(
        &gpu.device,
        &gpu.queue,
        OffscreenTarget::FORMAT,
        width,
        height,
    )
    .expect("the renderer builds");
    renderer
        .install(
            &gpu.device,
            &gpu.queue,
            &mut opened.extractor,
            &mut opened.assets,
        )
        .expect("the project installs");
    assert!(
        opened.assets.textures.is_empty(),
        "installing takes the textures it uploaded"
    );

    let target = OffscreenTarget::new(&gpu.device, width, height).expect("a target");
    renderer
        .draw(
            &gpu.device,
            &gpu.queue,
            target.view(),
            &opened.extractor,
            &opened.world,
        )
        .expect("the frame draws");
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let readback = target
        .copy_to_buffer(&gpu.device, &mut encoder)
        .expect("the target copies back");
    gpu.queue.submit([encoder.finish()]);
    let pixels = readback.read_rgba8(&gpu.device).expect("the frame reads");

    // A scene of grass, roads, a house and a lamp is many colors; a frame that
    // drew nothing, or drew one texture stretched over everything, is few.
    let mut colors: Vec<[u8; 3]> = pixels
        .chunks_exact(4)
        .step_by(97)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect();
    colors.sort_unstable();
    colors.dedup();
    assert!(
        colors.len() > 200,
        "the field draws as a picture, not a fill: {} colors",
        colors.len()
    );
}
