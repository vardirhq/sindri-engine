//! One run of a Sindri scene.
//!
//! A [`Session`] is what playing a scene is: its scripts, physics, screen UI,
//! effects, animations, sequences, saves, sound, and the scene changes a
//! script asks for, stepped at a fixed rate in the one order every host
//! shares. The shipped native and browser hosts drive it, and so does the
//! editor's Play, so a scene behaves the same in the editor as it does in the
//! build: that is the whole of why this crate exists, rather than each host
//! assembling its own loop from the engine's parts and drifting from the
//! others. See `docs/editor-update.md`.
//!
//! **There are no game rules here.** What a game does is in its Decay
//! scripts; this advances them and hands what they did to the engine.
//!
//! No window, GPU or editor: a host draws, and tells the session what it drew.

mod builtins;
mod error;
mod report;
mod session;
mod styling;

pub use builtins::bind_builtin_tile_sets;
pub use error::RuntimeError;
pub use report::{StepPhase, StepReport, StepTimes};
pub use session::Session;
