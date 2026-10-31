//! Authoring a scene on disk, for every editor host.
//!
//! The desktop editor and Sindri Mobile open, edit and save the same project
//! files, and a save made on one has to open on the other without conversion.
//! So the file side of authoring lives here, beneath both, rather than in
//! either: opening a scene or a prefab as a document, migrating an older one,
//! making each prefab instance from its prefab and standing in for one that
//! cannot be read, building the authored world, and writing it back as
//! canonical JSON.
//!
//! **No interface and no window.** What an editor draws, and how it decides
//! what to save, is the host's. A world reaches this crate only to be built
//! from a file or written to one, and every edit to it goes through the
//! engine's checked commands in between.

pub mod prefab;
pub mod scene_file;
mod world;

pub use scene_file::{SceneFile, SceneFileError, scene_path};
pub use world::{load_document, load_world};
