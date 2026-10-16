//! What differs between the native window and the browser: sound output and files.
//!
//! Both hosts offer the same two types. `Speaker` plays the live song and
//! one-shot effects; `Files` saves bytes and opens files. Natively a save is a
//! dialog and a write; in a browser it is a download, and an open arrives a
//! frame or more later, so both hosts deliver opened files through `poll`.

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
pub(super) use native::{Files, Speaker};
#[cfg(target_arch = "wasm32")]
pub(super) use web::{Files, Speaker};
#[cfg(target_arch = "wasm32")]
pub(in crate::lab) use web::{search, start};

/// Why a file was asked for, so the app knows what to do when it arrives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Purpose {
    Song,
    Midi,
    Sfx,
}

/// A file the person chose, read whole.
pub(super) struct Opened {
    pub(super) purpose: Purpose,
    pub(super) name: String,
    pub(super) bytes: Vec<u8>,
}

/// What a save asks for: the dialog title, a file name, and its type.
// A download has no dialog, so the browser reads only the name.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(super) struct SaveAs<'a> {
    pub(super) title: &'a str,
    pub(super) stem: &'a str,
    pub(super) extension: &'a str,
    pub(super) label: &'a str,
}

impl SaveAs<'_> {
    pub(super) fn file_name(&self) -> String {
        format!("{}.{}", self.stem, self.extension)
    }

    #[cfg(target_arch = "wasm32")]
    fn mime(&self) -> &'static str {
        match self.extension {
            "wav" => "audio/wav",
            "mid" => "audio/midi",
            _ => "application/json",
        }
    }
}
