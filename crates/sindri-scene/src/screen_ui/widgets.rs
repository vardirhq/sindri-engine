//! Authored widget values; visuals remain ordinary Weave-styled children.
use serde::Deserialize;
use sindri_core::SceneComponent;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct UiToggleComponent {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub checked: bool,
    #[serde(default)]
    pub disabled: bool,
}
impl SceneComponent for UiToggleComponent {
    const TYPE_NAME: &'static str = "sindri.ui.toggle";
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct UiTextInputComponent {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub placeholder: String,
    #[serde(default = "default_limit")]
    pub max_length: usize,
    #[serde(default)]
    pub disabled: bool,
}
const fn default_limit() -> usize {
    128
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

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct UiScrollComponent {
    #[serde(default)]
    pub label: String,
    /// Vertical content height in overlay units, independent of viewport height.
    #[serde(default)]
    pub content_height: f32,
    /// Distance down from the content's initial position, in overlay units.
    #[serde(default)]
    pub offset: f32,
    #[serde(default)]
    pub disabled: bool,
}
impl UiScrollComponent {
    #[must_use]
    pub fn coerce(&self, offset: f32, height: f32) -> f32 {
        if !offset.is_finite() || !self.content_height.is_finite() {
            return 0.0;
        }
        offset.clamp(0.0, (self.content_height - height.abs()).max(0.0))
    }
}
impl SceneComponent for UiScrollComponent {
    const TYPE_NAME: &'static str = "sindri.ui.scroll";
}

/// Host-neutral text and navigation input, supplied once per simulation step.
#[derive(Clone, Debug, Default)]
pub struct UiInput {
    pub text: String,
    pub next: bool,
    pub previous: bool,
    pub activate: bool,
    pub submit: bool,
    pub backspace: bool,
    pub escape: bool,
    pub blur: bool,
    pub scroll: f32,
}
