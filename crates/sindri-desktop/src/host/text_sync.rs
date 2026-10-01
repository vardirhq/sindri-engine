//! The host's side of text fields: handing the window's text entry to a field
//! that has the keyboard, and the system clipboard for copy and paste.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(not(target_arch = "wasm32"))]
use winit::event::WindowEvent;

#[cfg(not(target_arch = "wasm32"))]
use sindri_platform::InputEvent;

#[cfg(target_arch = "wasm32")]
use super::text_entry;
use super::{DesktopApp, Host, State};

impl<A: DesktopApp> Host<A> {
    /// Hands the window's text entry to a field that has the keyboard, and
    /// anything copied to the system clipboard.
    pub(super) fn sync_text_entry(&mut self) {
        let State::Running(running) = &mut self.state else {
            return;
        };
        let editing = running.app.editing_text();
        let copied = running.app.take_copied();
        if editing != self.editing_text {
            self.editing_text = editing;
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(window) = &self.window {
                window.set_ime_allowed(editing);
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            if self.text_entry.is_none() && self.window.is_some() {
                let canvas = self.config.canvas_id.as_ref().and_then(|id| {
                    web_sys::window()?
                        .document()?
                        .get_element_by_id(id)?
                        .dyn_into::<web_sys::HtmlElement>()
                        .ok()
                });
                self.text_entry = text_entry::TextEntry::new(&self.proxy, canvas);
            }
            if let Some(entry) = &self.text_entry {
                entry.set_editing(editing);
            }
            if let Some(text) = copied {
                text_entry::TextEntry::copy(&text);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(text) = copied
            && let Some(clipboard) = self.clipboard()
            && let Err(error) = clipboard.set_text(text)
        {
            log::warn!("could not copy to the clipboard: {error}");
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn clipboard(&mut self) -> Option<&mut arboard::Clipboard> {
        if self.clipboard.is_none() {
            match arboard::Clipboard::new() {
                Ok(clipboard) => self.clipboard = Some(clipboard),
                Err(error) => log::warn!("no system clipboard: {error}"),
            }
        }
        self.clipboard.as_mut()
    }

    /// Ctrl+V or Cmd+V in a text field: the clipboard's text, typed.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn paste(&mut self, event: &WindowEvent) {
        let WindowEvent::KeyboardInput { event, .. } = event else {
            return;
        };
        let shortcut = self.modifiers.control_key() || self.modifiers.super_key();
        if !shortcut
            || !self.editing_text
            || event.state != winit::event::ElementState::Pressed
            || event.physical_key
                != winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyV)
        {
            return;
        }
        let Some(text) = self.clipboard().and_then(|c| c.get_text().ok()) else {
            return;
        };
        if let State::Running(running) = &mut self.state {
            for c in text.chars().filter(|c| !c.is_control()) {
                running.app.input(InputEvent::TextInput(c));
            }
        }
    }
}
