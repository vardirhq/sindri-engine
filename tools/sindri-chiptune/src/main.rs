//! Standalone native chiptune workstation. No engine/editor dependencies.
#[cfg(not(target_arch = "wasm32"))]
mod desktop;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    desktop::run()
}

// Native audio and GUI are deliberately absent from the browser workspace build.
#[cfg(target_arch = "wasm32")]
fn main() {}
