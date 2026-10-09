//! Causeway: the companion game.
//!
//! Five orbs on a floor, a thing you drive with a keyboard or touch stick, and
//! a row of lamps that fills as you collect them. That is the whole game, and
//! it is the first thing built with this engine that someone can lose interest
//! in for the right reasons rather than the wrong ones.
//!
//! **There are no game rules in this file.** Moving, gathering, counting, and
//! winning are Decay scripts in `assets/scripts/`; this is a window, a device,
//! and a loop. That split is the claim the game exists to test: if authoring
//! gameplay meant writing Rust here, the scripting layer would not be doing its
//! job.
//!
//! The native build embeds the project so the standalone binary has no working
//! directory requirement. In a browser Causeway is a project like any other,
//! exported and played by `sindri-player`, which is the one host every export
//! ships.

#[cfg(not(target_arch = "wasm32"))]
use sindri_desktop::WindowConfig;

#[cfg(not(target_arch = "wasm32"))]
mod app;
mod assets;
mod error;
#[cfg(not(target_arch = "wasm32"))]
mod native_session;
/// Any project played offscreen, for the capture and benchmark tools.
#[cfg(not(target_arch = "wasm32"))]
pub mod project;

// The crate's public surface: what `bin/`, `tests/`, and the browser host
// reach for. Where an item lives inside the crate is not their business.
#[cfg(not(target_arch = "wasm32"))]
pub use assets::TEXTURES;
#[cfg(not(target_arch = "wasm32"))]
pub use assets::{
    AUDIO, FONTS, SHEETS, TEXTURE_IDS, TILE_SETS, bind_fonts, bind_textures, bind_tile_sets,
    scenes, sources, stylesheets, world,
};
pub use assets::{extractor, presented_world};
pub use error::CausewayError;
#[cfg(not(target_arch = "wasm32"))]
pub use native_session::session;
pub use sindri_player::bind_builtin_textures;
pub use sindri_runtime::Session;
pub use sindri_runtime::bind_builtin_tile_sets;

/// Opens the native game's window and plays it.
#[cfg(not(target_arch = "wasm32"))]
pub fn run() {
    env_logger::init();
    if let Err(error) = sindri_desktop::run::<app::CausewayApp>(WindowConfig {
        title: "Gather".to_owned(),
        ..WindowConfig::default()
    }) {
        log::error!("{error}");
    }
}
