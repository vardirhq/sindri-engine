//! The platform-neutral half of what the Android app shows: Lamplighter's
//! field opened as an authored world, and drawn by the engine.
//!
//! The draw is offscreen, at the Pixel 8's portrait resolution, through the
//! same `SceneRenderer` the app hands its surface to. It proves the frame is
//! built and encoded; that it reaches a phone's screen is the device check in
//! `mobile/first-slice.md`.

use std::path::{Path, PathBuf};

use sindri_core::{SceneEntityId, World};
use sindri_gpu::{GpuContext, GpuRequestOptions};
use sindri_mobile::{Editing, OpenedScene, SceneRenderer};
use sindri_render::OffscreenTarget;
use sindri_scene::CameraView;

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

/// The field opened and installed into a renderer drawing offscreen at the
/// Pixel 8's size, or `None` where there is no GPU to draw with.
struct Drawing {
    gpu: GpuContext,
    opened: OpenedScene,
    renderer: SceneRenderer,
    target: OffscreenTarget,
}

impl Drawing {
    fn new() -> Option<Self> {
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
                return None;
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
        let target = OffscreenTarget::new(&gpu.device, width, height).expect("a target");
        Some(Self {
            gpu,
            opened,
            renderer,
            target,
        })
    }

    /// `world` drawn through `camera`, lit or not, as RGBA bytes.
    fn draw(&mut self, world: Option<&World>, camera: CameraView, lit: bool) -> Vec<u8> {
        self.renderer.set_lit(lit);
        self.renderer
            .draw(
                &self.gpu.device,
                &self.gpu.queue,
                self.target.view(),
                &self.opened.extractor,
                world.unwrap_or(&self.opened.world),
                camera,
            )
            .expect("the frame draws");
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let readback = self
            .target
            .copy_to_buffer(&self.gpu.device, &mut encoder)
            .expect("the target copies back");
        self.gpu.queue.submit([encoder.finish()]);
        readback
            .read_rgba8(&self.gpu.device)
            .expect("the frame reads")
    }
}

#[test]
fn the_field_draws_at_phone_resolution() {
    let Some(mut drawing) = Drawing::new() else {
        return;
    };
    assert!(
        drawing.opened.assets.textures.is_empty(),
        "installing takes the textures it uploaded"
    );
    let brightness = |pixels: &[u8]| -> u64 {
        pixels
            .chunks_exact(4)
            .map(|pixel| u64::from(pixel[0]) + u64::from(pixel[1]) + u64::from(pixel[2]))
            .sum()
    };
    let unlit = drawing.draw(None, CameraView::default(), false);
    let pixels = drawing.draw(None, CameraView::default(), true);
    // Lamplighter is a night: its ambient is near black and its lamps light
    // the way. With the lighting preview off the field shows in its own
    // colours, which is far brighter than the lit night.
    assert!(
        brightness(&unlit) > brightness(&pixels) * 2,
        "the unlit field is brighter than the lit night"
    );

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

#[test]
fn the_selection_is_drawn_with_a_metal_edge_lit_or_not() {
    let Some(mut drawing) = Drawing::new() else {
        return;
    };
    let opened = &drawing.opened;
    let mut editing = Editing::new(&opened.world, &opened.extractor).expect("editing starts");
    let lamp = opened
        .world
        .entity_for_source_id(&SceneEntityId::new("cover-17".to_owned()).expect("a valid ID"))
        .expect("the field has cover-17");
    let at = opened
        .world
        .get(lamp)
        .and_then(|data| data.transform_3d)
        .expect("placed")
        .position;
    editing.canvas.center = [at[0], at[1]];
    editing.canvas.half_height = 3.0;
    editing.selected = Some(lamp);
    let outlined = editing.drawn(&opened.world, [1080.0, 2400.0], 4.0).clone();
    let canvas = editing.canvas.view();
    // The design's #FFBE55, which nothing else in the field is.
    let metal = |pixels: &[u8]| {
        pixels
            .chunks_exact(4)
            .filter(|pixel| {
                pixel[0] > 245 && (180..=200).contains(&pixel[1]) && (75..=95).contains(&pixel[2])
            })
            .count()
    };
    for lit in [false, true] {
        let plain = metal(&drawing.draw(None, canvas, lit));
        let edged = metal(&drawing.draw(Some(&outlined), canvas, lit));
        assert!(
            edged > plain + 500,
            "an edge is drawn round the selection, lit {lit}: {edged} metal pixels against {plain}"
        );
    }
}

#[test]
fn a_running_game_draws_through_the_same_renderer() {
    let Some(mut drawing) = Drawing::new() else {
        return;
    };
    let mut playing =
        sindri_mobile::Playing::start(&lamplighter(), "world.scene", [1080.0, 2400.0])
            .expect("the game starts");
    for _ in 0..30 {
        playing.advance(1.0 / 60.0).expect("the game steps");
    }
    playing
        .draw(
            &mut drawing.renderer,
            &drawing.gpu.device,
            &drawing.gpu.queue,
            drawing.target.view(),
        )
        .expect("the game draws");
    let mut encoder = drawing
        .gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let readback = drawing
        .target
        .copy_to_buffer(&drawing.gpu.device, &mut encoder)
        .expect("the target copies back");
    drawing.gpu.queue.submit([encoder.finish()]);
    let pixels = readback
        .read_rgba8(&drawing.gpu.device)
        .expect("the frame reads");
    let mut colors: Vec<[u8; 3]> = pixels
        .chunks_exact(4)
        .step_by(97)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect();
    colors.sort_unstable();
    colors.dedup();
    assert!(
        colors.len() > 100,
        "the running field draws as a picture: {} colors",
        colors.len()
    );
}
