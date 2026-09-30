//! Which window events are someone typing.
//!
//! A key press carries the text it would type whatever else is held, so
//! Ctrl+S arrives with an `s` and Cmd+A with an `a`. Those are shortcuts,
//! and a focused text field that took them as letters would fill with the
//! keys of every save. Alt is left alone: on many layouts it is `AltGr`,
//! which is how `@`, `€` and `{` are typed at all.

use winit::event::{ElementState, Ime, WindowEvent};
use winit::keyboard::ModifiersState;

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
    fn a_committed_composition_is_text_whatever_is_held() {
        let event = WindowEvent::Ime(Ime::Commit("日本".to_owned()));
        assert_eq!(typed(&event, ModifiersState::CONTROL), Some("日本"));
        assert_eq!(
            typed(&WindowEvent::Focused(true), ModifiersState::empty()),
            None
        );
    }
}
