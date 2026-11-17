//! `sindri`: a project checked, played and captured from a shell.
//!
//! The grammar is `sindri <noun> <verb> [target] [--flags]`, JSON is the
//! contract and the human form is rendered from it (`docs/cli-conventions.md`).
//! Commands read a project; they never change one. A coding agent changes a
//! project by editing its files, and uses this to see whether it worked
//! (`docs/agent-native-direction.md`).

// A shell tool: in the browser there is no shell, so the whole of it is
// native, and the browser build is an empty program rather than a failure.
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
mod check;
#[cfg(not(target_arch = "wasm32"))]
mod mcp;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
mod play;
#[cfg(not(target_arch = "wasm32"))]
mod report;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    native::main()
}
