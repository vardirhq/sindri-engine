//! `sindri.text`: words placed in the world like a sprite.
//!
//! Every field is `sindri.ui.text`'s, and means what it means there, with two
//! differences that come from where the text lives rather than what it is.
//! Sizes are in world units, so a damage number is as tall as the thing it
//! floats over whatever the camera's zoom. The anchor names which point of the
//! words sits on the entity, since a world entity has no viewport edge to be
//! anchored to. It is drawn with the sprites in its `layer`, over them.
//!
//! One field is its own: `follow_rotation`. Off, the words stay level however
//! the entity turns, which is what a damage number or a name plate wants. On,
//! they turn with it about the anchor point, for words painted on something.

use std::ops::{Deref, DerefMut};

use serde::Deserialize;
use sindri_core::SceneComponent;

use super::UiTextComponent;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct TextComponent {
    #[serde(flatten)]
    pub text: UiTextComponent,
    /// Whether the words turn with the entity's world rotation about the
    /// anchor point. Off, they stay level.
    #[serde(default)]
    pub follow_rotation: bool,
}

impl Deref for TextComponent {
    type Target = UiTextComponent;

    fn deref(&self) -> &UiTextComponent {
        &self.text
    }
}

impl DerefMut for TextComponent {
    fn deref_mut(&mut self) -> &mut UiTextComponent {
        &mut self.text
    }
}

impl SceneComponent for TextComponent {
    const TYPE_NAME: &'static str = "sindri.text";
}
