//! The native half of Sindri Mobile.
//!
//! The Android app is Kotlin and Jetpack Compose. Everything it shows of a
//! project comes from here, through the engine's own crates, so the phone sees
//! the same scene, with the same textures, as desktop does. The direction is in
//! `mobile/README.md` and progress in `mobile/first-slice.md`.
//!
//! Two parts, split by what they need:
//!
//! - [`OpenedScene`], [`Editing`], [`Canvas`] and [`SceneRenderer`] know nothing
//!   about Android. They open a project's scene, edit it by touch through the
//!   engine's checked commands, and draw it into any `wgpu` view, and they
//!   build and are tested on every desktop target.
//! - The `android` module is the JNI bridge the app calls. It turns an Android
//!   `Surface` into a `wgpu` surface and hands it frames, and it is the only
//!   code in this crate allowed to be unsafe.
//!
//! **A first slice of an editor.** One scene can be panned, pinched, selected,
//! moved, and have a field edited, with undo. Saving and playing come in later
//! slice steps, through the same code the desktop editor uses.

mod canvas;
#[cfg(not(target_arch = "wasm32"))]
mod editing;
#[cfg(not(target_arch = "wasm32"))]
mod opened;
mod renderer;

#[cfg(target_os = "android")]
mod android;

pub use canvas::Canvas;
#[cfg(not(target_arch = "wasm32"))]
pub use editing::Editing;
#[cfg(not(target_arch = "wasm32"))]
pub use opened::OpenedScene;
pub use renderer::SceneRenderer;
