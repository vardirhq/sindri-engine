//! Native cursor capture owned by editor Play, with checked window feedback.

use std::sync::Arc;

use eframe::egui;
use sindri_core::EngineState;
use winit::window::{CursorGrabMode, Window};

use crate::input::EditorInput;

use super::EditorApp;

#[derive(Default)]
pub(super) struct PlayCursor {
    window: Option<Arc<Window>>,
    locked: bool,
}

impl PlayCursor {
    pub(super) fn new(window: Option<Arc<Window>>) -> Self {
        Self {
            window,
            locked: false,
        }
    }

    fn request(&mut self, lock: bool, input: &mut EditorInput) -> Result<(), String> {
        if lock == self.locked {
            return Ok(());
        }
        let Some(window) = self.window.as_ref() else {
            return Err("cursor capture needs a native editor window".into());
        };
        if lock && !window.has_focus() {
            return Ok(());
        }
        let result = if lock {
            window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
        } else {
            window.set_cursor_grab(CursorGrabMode::None)
        };
        result.map_err(|error| format!("cursor capture: {error}"))?;
        self.locked = lock;
        window.set_cursor_visible(!lock);
        input.pointer_lock_changed(lock);
        Ok(())
    }
}

impl Drop for PlayCursor {
    fn drop(&mut self) {
        if self.locked
            && let Some(window) = self.window.as_ref()
        {
            let _ = window.set_cursor_grab(CursorGrabMode::None);
            window.set_cursor_visible(true);
        }
    }
}

impl EditorApp {
    fn capture_allowed(&self, context: &egui::Context) -> bool {
        self.lifecycle.state() == EngineState::Running
            && self.session.is_some()
            && self.game_view_rect.is_some()
            && !super::device::picking(context)
            && !context.egui_wants_keyboard_input()
            && context.input(|input| input.focused && !input.key_pressed(egui::Key::Escape))
    }

    /// Releases before input is sampled, and after layout/transport changes.
    pub(super) fn release_play_cursor_if_needed(&mut self, context: &egui::Context) {
        if !self.capture_allowed(context) {
            self.set_play_cursor(false);
        }
    }

    /// Window commands are drained after all fixed steps and editor actions.
    pub(super) fn drain_play_cursor(&mut self, context: &egui::Context) {
        let request = self
            .session
            .as_mut()
            .and_then(sindri_runtime::Session::take_pointer_lock_request);
        if let Some(lock) = request {
            // A script cannot capture from a panel, a hidden Game view or a
            // paused single step. Explicit release is always honored.
            let inside = self.input.state().pointer_locked() || self.game_pointer_available;
            if !lock || (inside && self.capture_allowed(context)) {
                self.set_play_cursor(lock);
            }
        }
        self.release_play_cursor_if_needed(context);
    }

    fn set_play_cursor(&mut self, lock: bool) {
        if let Err(problem) = self.play_cursor.request(lock, &mut self.input) {
            self.console.warning(problem);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_window_does_not_report_false_capture() {
        let mut cursor = PlayCursor::default();
        let mut input = EditorInput::default();
        assert!(cursor.request(true, &mut input).is_err());
        assert!(!input.state().pointer_locked());
        assert!(cursor.request(false, &mut input).is_ok());
    }
}
