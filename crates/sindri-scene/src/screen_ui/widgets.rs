//! Authored widget values; visuals remain ordinary Weave-styled children.
//!
//! A toggle, a text input and a scroll region each own what they mean — a flag,
//! a line of text, how far a list has moved — and nothing about how they look.
//! A checkbox is a toggle drawn as a box. What they draw is authored as their
//! children and styled by Weave, which reads `:checked`, `:disabled` and, from
//! [`crate::ScreenUi::focused`], `:focus`.
use serde::Deserialize;
use sindri_core::SceneComponent;

/// A flag a person flips: a switch, a checkbox, or -- given a `group` -- one
/// radio button of several.
///
/// Toggles that share a non-empty `group` are a radio group: pressing one
/// checks it and unchecks the rest, and pressing a checked one leaves it
/// checked, so the group always says which one was chosen.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct UiToggleComponent {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub checked: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub group: String,
}
impl SceneComponent for UiToggleComponent {
    const TYPE_NAME: &'static str = "sindri.ui.toggle";
}

/// A single line of text somebody types into.
///
/// Read through [`TextInputFields`], so a payload that cannot be typed into —
/// a limit of nothing, or past what a field is for, or a value already longer
/// than its limit — is refused where it is authored rather than clamped
/// somewhere nobody sees.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(try_from = "TextInputFields")]
pub struct UiTextInputComponent {
    pub label: String,
    pub value: String,
    pub placeholder: String,
    /// The most characters it holds, counted as Unicode scalars.
    pub max_length: usize,
    pub disabled: bool,
}

/// The most a text input may be authored to hold.
pub const MAX_TEXT_INPUT_LENGTH: usize = 4096;

/// A text input as authored, before it is checked.
#[derive(Deserialize)]
struct TextInputFields {
    #[serde(default)]
    label: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    placeholder: String,
    #[serde(default = "default_limit")]
    max_length: usize,
    #[serde(default)]
    disabled: bool,
}

const fn default_limit() -> usize {
    128
}

impl TryFrom<TextInputFields> for UiTextInputComponent {
    type Error = String;

    fn try_from(fields: TextInputFields) -> Result<Self, String> {
        if !(1..=MAX_TEXT_INPUT_LENGTH).contains(&fields.max_length) {
            return Err(format!(
                "max_length must be between 1 and {MAX_TEXT_INPUT_LENGTH}, not {}",
                fields.max_length
            ));
        }
        if fields.value.chars().count() > fields.max_length {
            return Err(format!(
                "value is longer than its max_length of {}",
                fields.max_length
            ));
        }
        Ok(Self {
            label: fields.label,
            value: fields.value,
            placeholder: fields.placeholder,
            max_length: fields.max_length,
            disabled: fields.disabled,
        })
    }
}
impl UiTextInputComponent {
    /// A single line, bounded by Unicode scalar count rather than byte count.
    #[must_use]
    pub fn coerce(&self, value: &str) -> String {
        value
            .chars()
            .filter(|c| !c.is_control())
            .take(self.max_length)
            .collect()
    }
}
impl SceneComponent for UiTextInputComponent {
    const TYPE_NAME: &'static str = "sindri.ui.text_input";
}

/// A region whose children scroll vertically and are clipped to it.
///
/// Read through [`ScrollFields`], so a height or offset that is negative or not
/// a number is refused where it is authored.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(try_from = "ScrollFields")]
pub struct UiScrollComponent {
    pub label: String,
    /// The least height the content has, in overlay units. What is laid out
    /// inside is measured, and a taller measure wins; zero leaves it at that.
    pub content_height: f32,
    /// Distance down from the content's initial position, in overlay units.
    pub offset: f32,
    pub disabled: bool,
}

/// A scroll region as authored, before it is checked.
#[derive(Deserialize)]
struct ScrollFields {
    #[serde(default)]
    label: String,
    #[serde(default)]
    content_height: f32,
    #[serde(default)]
    offset: f32,
    #[serde(default)]
    disabled: bool,
}

impl TryFrom<ScrollFields> for UiScrollComponent {
    type Error = String;

    fn try_from(fields: ScrollFields) -> Result<Self, String> {
        for (name, value) in [
            ("content_height", fields.content_height),
            ("offset", fields.offset),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!(
                    "{name} must be a number of zero or more, not {value}"
                ));
            }
        }
        Ok(Self {
            label: fields.label,
            content_height: fields.content_height,
            offset: fields.offset,
            disabled: fields.disabled,
        })
    }
}
impl UiScrollComponent {
    /// `offset` kept to what a region `height` tall can scroll.
    ///
    /// The content is as tall as the larger of what is authored and what the
    /// children `measured` as laid out, so a list that grows is never cut
    /// short by a stale number and a list authored taller keeps its room.
    #[must_use]
    pub fn coerce(&self, offset: f32, height: f32, measured: f32) -> f32 {
        let content = self.content_height.max(measured);
        if !offset.is_finite() || !content.is_finite() {
            return 0.0;
        }
        offset.clamp(0.0, (content - height.abs()).max(0.0))
    }
}
impl SceneComponent for UiScrollComponent {
    const TYPE_NAME: &'static str = "sindri.ui.scroll";
}

/// Host-neutral text and navigation input, supplied once per simulation step.
///
/// Each flag is an edge a host reports on its own, from its own key or pad
/// button, so they are kept as the separate facts they are rather than folded
/// into a set.
#[derive(Clone, Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct UiInput {
    /// Text committed this step: typed, composed by an IME, or pasted.
    pub text: String,
    pub next: bool,
    pub previous: bool,
    pub activate: bool,
    pub submit: bool,
    pub backspace: bool,
    pub escape: bool,
    pub blur: bool,
    pub scroll: f32,
    /// Arrow keys or a pad's d-pad: in a text field left and right move the
    /// caret, and everywhere else all four move focus to the nearest control
    /// that way.
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub home: bool,
    pub end: bool,
    pub delete: bool,
    /// Shift is held, so caret movement grows the selection.
    pub extend: bool,
    pub select_all: bool,
    pub copy: bool,
    pub cut: bool,
}
