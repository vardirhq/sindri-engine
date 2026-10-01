//! Window-independent input conversion for the shared scene UI runtime.
use sindri_platform::{GamepadAxis, GamepadButton, InputState, Key};
use sindri_scene::UiInput;

/// The step's keyboard and pads as the screen UI reads them.
///
/// A pad drives a menu as a keyboard does: the d-pad or the stick moves focus, South
/// presses (or submits a field), and East backs out as Escape. Any pad
/// counts, so a menu answers whichever one is picked up.
#[must_use]
pub fn ui_input(input: &InputState, viewport_height: f32) -> UiInput {
    let held = |keys: &[Key]| keys.iter().any(|key| input.key_down(*key));
    let shift = held(&[Key::ShiftLeft, Key::ShiftRight]);
    // Ctrl, or Command on a Mac: the shortcut key, whichever a platform uses.
    let command = held(&[
        Key::ControlLeft,
        Key::ControlRight,
        Key::SuperLeft,
        Key::SuperRight,
    ]);
    let pad = |button: GamepadButton| input.gamepads().pressed(0, button);
    UiInput {
        text: input.text_input().to_owned(),
        next: input.key_pressed(Key::Tab) && !shift,
        previous: input.key_pressed(Key::Tab) && shift,
        activate: input.key_pressed(Key::Space)
            || input.key_pressed(Key::Enter)
            || pad(GamepadButton::South),
        submit: input.key_pressed(Key::Enter) || pad(GamepadButton::South),
        backspace: input.key_pressed(Key::Backspace),
        escape: input.key_pressed(Key::Escape) || pad(GamepadButton::East),
        blur: !input.is_focused(),
        scroll: if viewport_height > 0.0 {
            input.scroll_delta()[1] * 2.0 / viewport_height
        } else {
            0.0
        },
        left: input.key_pressed(Key::ArrowLeft) || pad(GamepadButton::DPadLeft),
        right: input.key_pressed(Key::ArrowRight) || pad(GamepadButton::DPadRight),
        up: input.key_pressed(Key::ArrowUp) || pad(GamepadButton::DPadUp),
        down: input.key_pressed(Key::ArrowDown) || pad(GamepadButton::DPadDown),
        home: input.key_pressed(Key::Home),
        end: input.key_pressed(Key::End),
        delete: input.key_pressed(Key::Delete),
        extend: shift,
        select_all: command && input.key_pressed(Key::A),
        copy: command && input.key_pressed(Key::C),
        cut: command && input.key_pressed(Key::X),
        // A pad's Y grows downwards, as the screen's does; up is positive here.
        stick: [
            input.gamepads().axis(0, GamepadAxis::LeftX),
            -input.gamepads().axis(0, GamepadAxis::LeftY),
        ],
    }
}
