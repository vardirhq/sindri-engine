//! Text entry in a browser: IME composition, paste, and a phone's keyboard.
//!
//! A canvas cannot take text the way a page's own fields do. Its key events
//! carry no composed text, a paste never reaches it, and no phone shows a
//! keyboard for it. So while the game is editing a text field, a hidden
//! `<textarea>` holds the page's focus instead, and everything it hears is
//! handed to the game as the same input events a keyboard would make:
//! committed text from `input` and `compositionend` (typing, an IME, a
//! paste), and key presses for the keys that edit and navigate.
//!
//! Phones only show a keyboard for focus given during a tap, so the canvas's
//! own taps focus the textarea when the game is already editing, as well as
//! each frame asking for it.

use std::{cell::Cell, rc::Rc};

use sindri_platform::{InputEvent, Key};
use wasm_bindgen::{JsCast, closure::Closure};
use winit::event_loop::EventLoopProxy;

use super::startup::Startup;

pub(super) struct TextEntry {
    area: web_sys::HtmlTextAreaElement,
    canvas: Option<web_sys::HtmlElement>,
    editing: Rc<Cell<bool>>,
    _listeners: Vec<(
        web_sys::EventTarget,
        &'static str,
        Closure<dyn FnMut(web_sys::Event)>,
    )>,
}

impl TextEntry {
    pub(super) fn new(
        proxy: &EventLoopProxy<Startup>,
        canvas: Option<web_sys::HtmlElement>,
    ) -> Option<Self> {
        let document = web_sys::window()?.document()?;
        let area: web_sys::HtmlTextAreaElement =
            document.create_element("textarea").ok()?.dyn_into().ok()?;
        // On screen and focusable, so a phone will raise a keyboard for it,
        // and invisible, so nobody sees it.
        let _ = area.set_attribute(
            "style",
            "position:fixed;left:0;bottom:0;width:1px;height:1px;opacity:0;\
             border:0;padding:0;resize:none;font-size:16px;pointer-events:none;",
        );
        let _ = area.set_attribute("autocapitalize", "off");
        let _ = area.set_attribute("autocomplete", "off");
        let _ = area.set_attribute("spellcheck", "false");
        let _ = area.set_attribute("aria-hidden", "true");
        document.body()?.append_child(&area).ok()?;

        let editing = Rc::new(Cell::new(false));
        let mut entry = Self {
            area: area.clone(),
            canvas: canvas.clone(),
            editing: Rc::clone(&editing),
            _listeners: Vec::new(),
        };
        let send = {
            let proxy = proxy.clone();
            move |event: InputEvent| {
                if proxy.send_event(Startup::Input(event)).is_err() {
                    log::debug!("text arrived after the event loop closed");
                }
            }
        };
        let commit = {
            let area = area.clone();
            let send = send.clone();
            move || {
                for c in area.value().chars().filter(|c| !c.is_control()) {
                    send(InputEvent::TextInput(c));
                }
                area.set_value("");
            }
        };
        {
            let commit = commit.clone();
            entry.listen(area.clone().into(), "input", move |event| {
                let composing = event
                    .dyn_ref::<web_sys::InputEvent>()
                    .is_some_and(web_sys::InputEvent::is_composing);
                if !composing {
                    commit();
                }
            });
        }
        entry.listen(area.clone().into(), "compositionend", move |_| commit());
        {
            let send = send.clone();
            entry.listen(area.clone().into(), "keydown", move |event| {
                let Some(key_event) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
                    return;
                };
                // A key that edits or moves is the game's; a newline or a
                // tab must not land in the textarea or move the page's focus.
                if matches!(key_event.key().as_str(), "Tab" | "Enter") {
                    event.prevent_default();
                }
                if !key_event.is_composing()
                    && let Some(key) = key_of(&key_event.code())
                {
                    send(InputEvent::KeyPressed(key));
                }
            });
        }
        {
            let send = send.clone();
            entry.listen(area.clone().into(), "keyup", move |event| {
                if let Some(key) = event
                    .dyn_ref::<web_sys::KeyboardEvent>()
                    .and_then(|key_event| key_of(&key_event.code()))
                {
                    send(InputEvent::KeyReleased(key));
                }
            });
        }
        {
            // Focus leaving for anywhere but the game is the window losing it.
            let canvas = canvas.clone();
            let send = send.clone();
            entry.listen(area.clone().into(), "blur", move |event| {
                let to_canvas = event
                    .dyn_ref::<web_sys::FocusEvent>()
                    .and_then(web_sys::FocusEvent::related_target)
                    .zip(canvas.as_ref())
                    .is_some_and(|(target, canvas)| {
                        target == web_sys::EventTarget::from(canvas.clone())
                    });
                if !to_canvas {
                    send(InputEvent::FocusChanged(false));
                }
            });
        }
        if let Some(canvas) = canvas {
            let area = area.clone();
            let editing = Rc::clone(&editing);
            entry.listen(canvas.into(), "pointerup", move |_| {
                if editing.get() {
                    let _ = area.focus();
                }
            });
        }
        Some(entry)
    }

    fn listen(
        &mut self,
        target: web_sys::EventTarget,
        name: &'static str,
        handler: impl FnMut(web_sys::Event) + 'static,
    ) {
        let callback = Closure::wrap(Box::new(handler) as Box<dyn FnMut(web_sys::Event)>);
        if target
            .add_event_listener_with_callback(name, callback.as_ref().unchecked_ref())
            .is_ok()
        {
            self._listeners.push((target, name, callback));
        }
    }

    /// Whether the textarea, rather than the canvas, has the page's focus:
    /// the canvas losing focus to it is not the window losing focus.
    pub(super) fn has_focus(&self) -> bool {
        web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.active_element())
            .is_some_and(|active| active == web_sys::Element::from(self.area.clone()))
    }

    /// Gives the textarea the page's focus while the game edits text, and
    /// hands it back to the canvas when it stops.
    pub(super) fn set_editing(&self, editing: bool) {
        if self.editing.replace(editing) == editing {
            return;
        }
        if editing {
            let _ = self.area.focus();
        } else {
            self.area.set_value("");
            if self.has_focus() {
                if let Some(canvas) = &self.canvas {
                    let _ = canvas.focus();
                } else {
                    let _ = self.area.blur();
                }
            }
        }
    }

    /// Puts copied text on the system clipboard, where the page allows it.
    pub(super) fn copy(text: &str) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let clipboard = js_sys::Reflect::get(&window.navigator(), &"clipboard".into());
        let Ok(clipboard) = clipboard else { return };
        let write = js_sys::Reflect::get(&clipboard, &"writeText".into())
            .ok()
            .and_then(|f| f.dyn_into::<js_sys::Function>().ok());
        if let Some(write) = write {
            // A promise nobody awaits: a refused write leaves the clipboard
            // as it was, which is all a copy can promise in a page.
            let _ = write.call1(&clipboard, &text.into());
        }
    }
}

impl Drop for TextEntry {
    fn drop(&mut self) {
        for (target, name, callback) in &self._listeners {
            let _ =
                target.remove_event_listener_with_callback(name, callback.as_ref().unchecked_ref());
        }
        self.area.remove();
    }
}

/// The engine's key for a DOM `code`, which names the physical key as the
/// engine does: `KeyA` is `A`, `ShiftLeft` is `ShiftLeft`.
fn key_of(code: &str) -> Option<Key> {
    let name = match code {
        "MetaLeft" | "OSLeft" => "SuperLeft",
        "MetaRight" | "OSRight" => "SuperRight",
        "NumpadEnter" => "Enter",
        other => other.strip_prefix("Key").unwrap_or(other),
    };
    Key::from_name(name)
}
