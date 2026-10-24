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
//! down the list), `key:<Key>` taps a key, `hold:<Key>` and `release:<Key>`
//! press one across the steps between, `type:<text>` commits characters,
//! `set:<name>=<value>` writes a shared board value, `move:<x>,<y>` puts the
//! pointer at a pixel, `down` and `up` press and release the left button
//! there, and `wait:<seconds>` plays on. Each prints what it hit, so a picture of the
//! wrong thing says why.

#[cfg(not(target_arch = "wasm32"))]
mod capture {
    use std::{error::Error, path::Path};

    use sindri_causeway::project::{open, photograph};

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
        photograph(&mut opened, (width, height), Path::new(out)).await?;
        println!("wrote {out}");
        Ok(())
    }

    #[cfg(test)]
    #[test]
    fn native_project_delivery_expands_prefabs_and_resolves_physics_profiles() {
        use sindri_core::SceneEntityId;

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../games/lamplighter");
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
