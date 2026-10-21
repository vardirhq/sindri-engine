//! A deterministic picture of Causeway being played, offscreen.
//!
//! The window is not always available and never reproducible; this is. It
//! opens the project in the same session the window plays, begins a crossing
//! from the title, lets the first Hollow climb out and choose their strikes,
//! and takes up the first warden -- so the picture shows the board as a player
//! meets it: figures standing, intents marked on the ground, a move offered.
//!
//! ```bash
//! cargo run -p sindri-causeway --bin causeway-capture -- target/causeway.png [width] [height] [steps...]
//! ```
//!
//! Any further arguments are steps, as `project-capture` reads them, played
//! after the battle has begun.

#[cfg(not(target_arch = "wasm32"))]
mod capture {
    use std::{error::Error, path::Path};

    use sindri_causeway::project::{open, photograph};

    pub(super) async fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
        env_logger::builder()
            .filter_level(log::LevelFilter::Warn)
            .try_init()
            .ok();
        let mut args = args.into_iter();
        let out = args
            .next()
            .ok_or("causeway-capture <out.png> [width] [height] [steps...]")?;
        let width: u32 = args.next().map_or(Ok(1280), |w| w.parse())?;
        let height: u32 = args.next().map_or(Ok(720), |h| h.parse())?;
        #[allow(clippy::cast_precision_loss)]
        let size = [width as f32, height as f32];
        let mut opened = open(&sindri_causeway::project_dir(), size)?;
        opened.player.play(0.5)?;
        // From the title into the first island, and long enough for the
        // Hollow to climb out, walk and choose.
        opened.player.perform("key:Enter")?;
        opened.player.play(6.0)?;
        for step in args {
            opened.player.perform(&step)?;
        }
        opened.player.play(0.4)?;
        photograph(&mut opened, (width, height), Path::new(&out)).await?;
        println!("wrote {out}");
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(capture::run(std::env::args().skip(1).collect()))
}

#[cfg(target_arch = "wasm32")]
fn main() {}
