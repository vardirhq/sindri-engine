//! Where a session's save is kept, and when it is written out.

use super::Session;

impl Session {
    /// Keeps this session's save somewhere the host chose, loading what is
    /// already there.
    ///
    /// Called before the first frame: a game that read its progress after
    /// starting would have already begun a run without it.
    pub fn keep_saves_in(&mut self, mut backend: Box<dyn sindri_platform::SaveBackend>) {
        self.saves = sindri_core::SaveStore::opened(backend.read());
        self.save_backend = backend;
        self.since_written = 0.0;
    }

    /// Writes the save out if anything changed and enough time has passed.
    ///
    /// A failure is reported and does not stop the frame: a disk that will not
    /// take a save is worth knowing about, and it is not a reason to end
    /// someone's run.
    pub(super) fn write_saves(&mut self, elapsed: f32, force: bool) {
        self.since_written += elapsed;
        if !self.saves.is_dirty() || (!force && self.since_written < SAVE_INTERVAL_SECONDS) {
            return;
        }
        self.since_written = 0.0;
        match self.save_backend.write(&self.saves.to_document()) {
            Ok(()) => self.saves.mark_written(),
            // Left dirty on purpose, so the next attempt tries again rather
            // than believing a write that did not happen.
            Err(error) => log::error!("the save could not be written: {error}"),
        }
    }
}

/// How long a change waits before being written out.
///
/// Long enough that a value changing every frame does not keep a disk busy,
/// short enough that a browser tab closing loses almost nothing.
const SAVE_INTERVAL_SECONDS: f32 = 2.0;
