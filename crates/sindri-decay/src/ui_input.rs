//! Window-independent input conversion for the shared scene UI runtime.
use sindri_platform::{InputState, Key};
use sindri_scene::UiInput;

#[must_use]
pub fn ui_input(input: &InputState, viewport_height: f32) -> UiInput {
    let shift = input.key_down(Key::ShiftLeft) || input.key_down(Key::ShiftRight);
    UiInput {
        text: input.text_input().to_owned(),
        next: input.key_pressed(Key::Tab) && !shift,
        previous: input.key_pressed(Key::Tab) && shift,
        activate: input.key_pressed(Key::Space) || input.key_pressed(Key::Enter),
        submit: input.key_pressed(Key::Enter),
        backspace: input.key_pressed(Key::Backspace),
        escape: input.key_pressed(Key::Escape),
        blur: !input.is_focused(),
        scroll: if viewport_height > 0.0 { input.scroll_delta()[1] * 2.0 / viewport_height } else { 0.0 },
    }
}
