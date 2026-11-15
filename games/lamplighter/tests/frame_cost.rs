//! What one frame of each scene costs to prepare: how many sprites it hands
//! the renderer, how many lights it carries, and how long extraction takes.
//!
//! Ignored by default: it measures rather than checks. Run it with
//! `cargo test --release -p lamplighter --test frame_cost -- --ignored --nocapture`.

use std::time::Instant;

use lamplighter::Run;
use sindri_render::{FrameCommand, Viewport};
use sindri_scene::{CameraView, SceneRuntime, TextureBindings};

/// How many frames each scene is extracted for.
const FRAMES: u32 = 60;

fn measure(scene: &str) {
    let mut run = Run::open_scene(scene).expect("the scene opens");
    for _ in 0..3 {
        let _ = run.step(1.0 / 60.0);
    }
    let textures = TextureBindings::new();
    let viewport = Viewport {
        x: 0,
        y: 0,
        width: 1080,
        height: 2400,
    };
    let (sprites, lights, seconds) = run
        .draw_with(|extractor, world, session| {
            // The project's block sets, and the engine's own, as the editor
            // binds them.
            let mut tile_sets = session.tile_sets().clone();
            for builtin in sindri_assets::builtin_tile_sets() {
                let (id, set) = builtin.map_err(|error| error.to_string())?;
                tile_sets.bind(id, set).map_err(|error| error.to_string())?;
            }
            let mut sprites = 0;
            let mut lights = 0;
            let started = Instant::now();
            for _ in 0..FRAMES {
                let frame = extractor
                    .extract_animated(
                        world,
                        viewport,
                        CameraView::default(),
                        &textures,
                        SceneRuntime {
                            animations: Some(session.animations()),
                            effects: Some(session.effects()),
                            tile_sets: Some(&tile_sets),
                            ..SceneRuntime::default()
                        },
                    )
                    .map_err(|error| error.to_string())?;
                sprites = frame
                    .passes()
                    .iter()
                    .map(|pass| match &pass.command {
                        FrameCommand::SpriteBatch { instances, .. } => instances.len(),
                        _ => 0,
                    })
                    .sum::<usize>();
                lights = frame
                    .sprite_environment()
                    .lighting
                    .as_ref()
                    .map_or(0, |lighting| lighting.lights.len());
            }
            let seconds = started.elapsed().as_secs_f64() / f64::from(FRAMES);
            Ok(((sprites, lights, seconds), sindri_scene::UiTextSizes::new()))
        })
        .expect("a frame");
    println!(
        "{scene}: {sprites} sprite instances, {lights} lights, {:.2} ms to extract",
        seconds * 1000.0
    );
}

#[test]
#[ignore = "a measurement, not a check"]
fn frame_cost() {
    for scene in ["world.scene", "village.scene"] {
        measure(scene);
    }
}
