//! The input actions a scene declares: what a person means, bound to what
//! they press.
//!
//! The scene holds the declaration as data and nothing more. What a binding
//! means is `sindri-platform`'s `ActionMap`, which reads this payload as an
//! actions document, and scripts reach it as `Action`. Held on the scene, as
//! its physics world's layer names are, so every host, export and the editor
//! already carry it, and a rebinding a game makes while it runs is a write to
//! this component.

use serde::Deserialize;
use sindri_core::SceneComponent;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct InputActionsComponent {
    /// Each action in the actions-document shape: `name`, `kind` (button, axis
    /// or vector) and `bindings`. The order is the order ids are handed out.
    #[serde(default)]
    pub actions: Vec<serde_json::Value>,
}

impl SceneComponent for InputActionsComponent {
    const TYPE_NAME: &'static str = "sindri.input.actions";
}
