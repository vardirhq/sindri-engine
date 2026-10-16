//! The native Sound Lab: a window, or a headless render when given arguments.
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    sindri_chiptune::run()
}

// The browser build starts from the library's `start`, not from a binary.
#[cfg(target_arch = "wasm32")]
fn main() {}
