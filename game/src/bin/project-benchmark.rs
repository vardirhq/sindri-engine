//! Times any project played the way the browser plays it, for comparison
//! with the editor playing the same project.
//!
//! ```bash
//! cargo run -p sindri-causeway --bin project-benchmark -- \
//!     games/platformer target/bench/platformer-standalone.json [frames] [settle]
//! ```
//!
//! Each frame is one fixed step and one draw, offscreen at 1280×720: the same
//! session, styling, extraction and encoding a host runs, without a window to
//! present to. The GPU is waited on after each frame so one frame's drawing is
//! not timed as the next one's work, and that wait is reported as `gpu` apart
//! from the CPU phases. The report has the editor benchmark's shape — see
//! `sindri_editor::benchmark` — with a `playing` section only, because a
//! standalone game has nothing to edit. `scripts/frame-benchmark.py` reads
//! both.

#[cfg(not(target_arch = "wasm32"))]
mod benchmark {
    use std::{error::Error, fs, path::Path, time::Instant};

    use serde_json::{Value, json};
    use sindri_causeway::project::{ProjectRenderer, open};
    use sindri_gpu::{GpuContext, GpuRequestOptions};

    const SIZE: (u32, u32) = (1280, 720);

    fn micros(duration: std::time::Duration) -> u64 {
        u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
    }

    pub(super) async fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
        let (project, report, frames, settle) = match args.as_slice() {
            [project, report] => (project, report, 600, 60),
            [project, report, frames] => (project, report, frames.parse()?, 60),
            [project, report, frames, settle] => {
                (project, report, frames.parse()?, settle.parse()?)
            }
            _ => {
                return Err("project-benchmark <project> <report.json> [frames] [settle]".into());
            }
        };
        #[allow(clippy::cast_precision_loss)]
        let size = [SIZE.0 as f32, SIZE.1 as f32];
        let mut opened = open(Path::new(project), size)?;
        let instance = wgpu::Instance::default();
        let gpu = GpuContext::request(&instance, None, &GpuRequestOptions::default()).await?;
        let mut renderer = ProjectRenderer::new(&gpu, &opened.images, &opened.sheets, SIZE)?;
        let mut playing: Vec<Value> = Vec::with_capacity(frames);
        for at in 0..settle + frames {
            let began = Instant::now();
            opened.player.advance()?;
            let stepped = began.elapsed();
            let drawn = renderer.draw(&mut opened.player, &gpu)?;
            let waiting = Instant::now();
            gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
            let gpu_time = waiting.elapsed();
            if at >= settle {
                playing.push(json!({
                    "steps": 1,
                    "phases": {
                        "step": micros(stepped),
                        "presentation": micros(drawn.presentation),
                        "extraction": micros(drawn.extraction),
                        "encoding": micros(drawn.encoding),
                        "gpu": micros(gpu_time),
                    },
                }));
            }
        }
        let written = json!({
            "host": "standalone",
            "opened": project,
            "optimized": !cfg!(debug_assertions),
            "window": [SIZE.0, SIZE.1],
            "sections": { "playing": playing },
        });
        if let Some(parent) = Path::new(report).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(report, serde_json::to_string_pretty(&written)?)?;
        println!("Benchmark: {frames} playing frames written to {report}");
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(benchmark::run(std::env::args().skip(1).collect()))
}

#[cfg(target_arch = "wasm32")]
fn main() {}
