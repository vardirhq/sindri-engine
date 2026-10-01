//! Which window events are someone typing.
//!
//! A key press carries the text it would type whatever else is held, so
//! Ctrl+S arrives with an `s` and Cmd+A with an `a`. Those are shortcuts,
//! and a focused text field that took them as letters would fill with the
//! keys of every save. Alt is left alone: on many layouts it is `AltGr`,
//! which is how `@`, `€` and `{` are typed at all.

use winit::event::{ElementState, Ime, WindowEvent};
use winit::keyboard::ModifiersState;

/// What a window's key presses and IME have typed lately, so text is
/// committed once.
///
/// With an IME allowed, some platforms report a key's character both on its
/// press and as an IME commit; and while a composition is open, the keys
/// building it are not text yet. A commit that repeats the press before it
/// is dropped, and a press during a composition types nothing.
#[derive(Debug, Default)]
pub(super) struct Typist {
    composing: bool,
    pressed: Option<String>,
}

impl Typist {
    /// The text `event` commits, if any, once.
    pub(super) fn typed(
        &mut self,
        event: &WindowEvent,
        modifiers: ModifiersState,
    ) -> Option<String> {
        match event {
            WindowEvent::Ime(Ime::Preedit(text, _)) => {
                self.composing = !text.is_empty();
                None
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                self.composing = false;
                if self.pressed.take().as_deref() == Some(text.as_str()) {
                    return None;
                }
                Some(text.clone())
            }
            WindowEvent::KeyboardInput { .. } if self.composing => None,
            _ => {
                let text = typed(event, modifiers).map(str::to_owned);
                if text.is_some() {
                    self.pressed.clone_from(&text);
                }
                text
            }
        }
    }
}

/// The committed text `event` types, if it types any.
pub(super) fn typed(event: &WindowEvent, modifiers: ModifiersState) -> Option<&str> {
    match event {
        WindowEvent::KeyboardInput { event, .. }
            if event.state == ElementState::Pressed && !is_shortcut(modifiers) =>
        {
            event.text.as_deref()
        }
        WindowEvent::Ime(Ime::Commit(text)) => Some(text.as_str()),
        _ => None,
    }
}

/// Whether the held modifiers make a key press a shortcut rather than a
/// letter.
fn is_shortcut(modifiers: ModifiersState) -> bool {
    modifiers.control_key() || modifiers.super_key()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_and_command_are_shortcuts_and_shift_and_alt_are_typing() {
        assert!(is_shortcut(ModifiersState::CONTROL));
        assert!(is_shortcut(ModifiersState::SUPER));
        assert!(!is_shortcut(ModifiersState::SHIFT));
        assert!(!is_shortcut(ModifiersState::ALT));
        assert!(!is_shortcut(ModifiersState::empty()));
    }

    #[test]
    fn a_commit_repeating_the_key_press_before_it_types_once() {
        use winit::event::Ime;
        let mut typist = Typist::default();
        let commit = WindowEvent::Ime(Ime::Commit("a".to_owned()));
        typist.pressed = Some("a".to_owned());
        assert_eq!(typist.typed(&commit, ModifiersState::empty()), None);
        assert_eq!(
            typist.typed(&commit, ModifiersState::empty()).as_deref(),
            Some("a"),
            "a commit with no press before it is text"
        );
        let preedit = WindowEvent::Ime(Ime::Preedit("に".to_owned(), None));
        assert_eq!(typist.typed(&preedit, ModifiersState::empty()), None);
        assert!(typist.composing);
    }

    #[test]
    fn a_committed_composition_is_text_whatever_is_held() {
        let event = WindowEvent::Ime(Ime::Commit("日本".to_owned()));
        assert_eq!(typed(&event, ModifiersState::CONTROL), Some("日本"));
        assert_eq!(
            typed(&WindowEvent::Focused(true), ModifiersState::empty()),
            None
        );
    }
}
