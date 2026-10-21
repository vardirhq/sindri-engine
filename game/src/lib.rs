//! Causeway: the showcase.
//!
//! A tactics game on islands joined by causeways. The Hollow cross the
//! causeways at low tide; three wardens hold each island until the sea comes
//! in. Everything that game is lives in `assets/`: the rules and the turn in
//! Decay scripts, the screens in Weave, the islands and the figures as models
//! built by `tools/causeway_art.py`.
//!
//! **There are no game rules in this crate.** It opens the project in the one
//! host every export ships, and keeps the offscreen tools -- the capture and
//! the benchmark -- that play any project the way that host does.

/// Any project played offscreen, for the capture and benchmark tools.
#[cfg(not(target_arch = "wasm32"))]
pub mod project;

pub use sindri_player::bind_builtin_textures;
pub use sindri_runtime::Session;
pub use sindri_runtime::bind_builtin_tile_sets;

/// The scene schemas a Causeway session reads, scripts included: the
/// registry the offscreen tools and the tests open projects with.
///
/// # Errors
/// A component that will not register, which is a broken build.
pub fn extractor() -> Result<sindri_scene::SceneExtractor, String> {
    sindri_runtime::scene_extractor()
}

/// Where the project lives: the crate's own folder, which is what makes
/// `cargo run -p sindri-causeway` play it from any working directory.
#[must_use]
pub fn project_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Opens the game's window and plays it, in the host every export ships.
#[cfg(not(target_arch = "wasm32"))]
pub fn run() {
    sindri_player::run_project(project_dir(), "Causeway");
}
