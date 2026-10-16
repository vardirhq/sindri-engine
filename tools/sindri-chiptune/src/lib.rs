//! Sindri Sound Lab: game sound effects and chiptune music.
//!
//! One crate, two hosts. Natively it is a window and a command line that
//! deliver sounds straight into a Sindri project; in a browser it is the same
//! window, with downloads in place of the project folder. It depends on no
//! engine crate in either.
mod lab;

/// Opens the Sound Lab window, or renders headless when given arguments.
///
/// # Errors
/// A command-line render that cannot read its input or write its output, or a
/// window that cannot open.
#[cfg(not(target_arch = "wasm32"))]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    lab::run()
}

/// The browser entry point: starts the Sound Lab on the page's canvas.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    lab::start_web();
}
