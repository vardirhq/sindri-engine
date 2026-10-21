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
mod models;
mod player;
mod project;

pub use builtins::bind_builtin_textures;
pub use error::PlayerError;
pub use project::ProjectAssets;

use sindri_desktop::WindowConfig;

/// The project a native game crate asked to play, when it is not named on
/// the command line.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) static PROJECT: std::sync::OnceLock<(std::path::PathBuf, String)> =
    std::sync::OnceLock::new();

/// Opens a window titled `title` and plays the project in `directory`.
///
/// For a game that ships as its own binary: the game's crate names its
/// project folder and gets the same host every export runs, rather than
/// assembling a window, a session and a renderer of its own that drift from
/// what the player does.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_project(directory: impl Into<std::path::PathBuf>, title: &str) {
    let _ = PROJECT.set((directory.into(), title.to_owned()));
    run();
}

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
    #[cfg(not(target_arch = "wasm32"))]
    let title = PROJECT
        .get()
        .map_or_else(|| "Sindri".to_owned(), |(_, title)| title.clone());
    #[cfg(target_arch = "wasm32")]
    let title = "Sindri".to_owned();
    if let Err(error) = sindri_desktop::run::<player::Player>(WindowConfig {
        title,
        ..WindowConfig::default()
    }) {
        log::error!("{error}");
        #[cfg(not(target_arch = "wasm32"))]
        eprintln!("sindri-player: {error}");
    }
}
