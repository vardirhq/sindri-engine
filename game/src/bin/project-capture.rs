//! A photograph of any project, offscreen, played the way the browser plays
//! it: the same session, the same Weave styling for the viewport, the same
//! draw, with the person's clicks, keys and wheel delivered as a host would.
//!
//! ```bash
//! cargo run -p sindri-causeway --bin project-capture -- \
//!     examples/ui target/ui.png 390 844 click:callsign type:Nova key:Enter wait:0.5
//! ```
//!
//! Steps after the size run in order: `click:<entity>` presses and releases
//! over an element, `wheel:<entity>:<pixels>` scrolls over one (positive is
//! down the list), `key:<Key>` taps a key, `type:<text>` commits characters,
//! `set:<name>=<value>` writes a shared board value, `move:<x>,<y>` puts the
//! pointer at a pixel, `down` and `up` press and release the left button
//! there, and `wait:<seconds>` plays on. Each prints what it hit, so a picture of the
//! wrong thing says why.

#[cfg(not(target_arch = "wasm32"))]
mod capture {
    use std::{error::Error, fs, io::BufWriter, path::Path};

    use sindri_causeway::project::{OpenedProject, ProjectRenderer, open};
    use sindri_gpu::{GpuContext, GpuRequestOptions};

    pub(super) async fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
        env_logger::builder()
            .filter_level(log::LevelFilter::Info)
            .try_init()
            .ok();
        let [project, out, width, height, steps @ ..] = args.as_slice() else {
            return Err("project-capture <project> <out.png> <width> <height> [steps...]".into());
        };
        let (width, height): (u32, u32) = (width.parse()?, height.parse()?);
        #[allow(clippy::cast_precision_loss)]
        let size = [width as f32, height as f32];
        let mut opened = open(Path::new(project), size)?;
        opened.player.play(0.5)?;
        for step in steps {
            opened.player.perform(step)?;
        }
        opened.player.play(0.3)?;
        photograph(&mut opened, (width, height), Path::new(out)).await
    }

    /// Draws the player's world once, offscreen, and writes it to `path`.
    async fn photograph(
        opened: &mut OpenedProject,
        size: (u32, u32),
        path: &Path,
    ) -> Result<(), Box<dyn Error>> {
        let instance = wgpu::Instance::default();
        let gpu = GpuContext::request(&instance, None, &GpuRequestOptions::default()).await?;
        let mut renderer = ProjectRenderer::new(&gpu, &opened.images, &opened.sheets, size)?;
        renderer.draw(&mut opened.player, &gpu)?;
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("project capture readback"),
            });
        let readback = renderer
            .target()
            .copy_to_buffer(&gpu.device, &mut encoder)?;
        gpu.queue.submit([encoder.finish()]);
        let pixels = readback.read_rgba8(&gpu.device)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut png = png::Encoder::new(BufWriter::new(fs::File::create(path)?), size.0, size.1);
        png.set_color(png::ColorType::Rgba);
        png.set_depth(png::BitDepth::Eight);
        png.write_header()?.write_image_data(&pixels)?;
        println!("wrote {}", path.display());
        Ok(())
    }

    #[cfg(test)]
    #[test]
    fn native_project_delivery_expands_prefabs_and_resolves_physics_profiles() {
        use sindri_core::SceneEntityId;

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../games/platformer");
        let mut opened = open(&root, [960.0, 540.0]).unwrap();
        opened.player.play(0.5).unwrap();
        let crate_id = SceneEntityId::new("wind-crate").unwrap();
        assert!(
            opened
                .player
                .world
                .entity_for_source_id(&crate_id)
                .is_some()
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(capture::run(std::env::args().skip(1).collect()))
}

#[cfg(target_arch = "wasm32")]
fn main() {}
