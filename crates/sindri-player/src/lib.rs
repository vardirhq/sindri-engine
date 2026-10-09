//! Plays any Sindri project.
//!
//! The one host a project needs and nothing project-specific: in a browser
//! it is the WebAssembly module every export ships (`sindri_player.js`),
//! fetching the project the export wrote beside it; natively it is
//! `sindri-player <project>`, reading the project from its directory. Both
//! fill the same [`ProjectAssets`] and play them the same way, on the shared
//! runtime session every host steps, so a project behaves the same wherever
//! it is played.
//!
//! **There are no game rules here.** What a game does is in its scenes and
//! its Decay scripts.

#[cfg(target_arch = "wasm32")]
mod browser;
mod builtins;
#[cfg(not(target_arch = "wasm32"))]
mod directory;
mod error;
mod player;
mod project;

pub use builtins::bind_builtin_textures;
pub use error::PlayerError;
pub use project::ProjectAssets;

use sindri_desktop::WindowConfig;

/// Opens the window and plays the project: the one served beside the page
/// in a browser, the directory named on the command line natively.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(start))]
pub fn run() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        let _ = console_log::init_with_level(log::Level::Info);
    }
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();
    if let Err(error) = sindri_desktop::run::<player::Player>(WindowConfig {
        title: "Sindri".to_owned(),
        ..WindowConfig::default()
    }) {
        log::error!("{error}");
        #[cfg(not(target_arch = "wasm32"))]
        eprintln!("sindri-player: {error}");
    }
}
