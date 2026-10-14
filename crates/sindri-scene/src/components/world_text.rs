//! `sindri.text`: words placed in the world like a sprite.
//!
//! Every field is `sindri.ui.text`'s, and means what it means there, with two
//! differences that come from where the text lives rather than what it is.
//! Sizes are in world units, so a damage number is as tall as the thing it
//! floats over whatever the camera's zoom. The anchor names which point of the
//! words sits on the entity, since a world entity has no viewport edge to be
//! anchored to. It is drawn with the sprites in its `layer`, over them.

use std::ops::{Deref, DerefMut};

use serde::Deserialize;
use sindri_core::SceneComponent;

use super::UiTextComponent;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct TextComponent(pub UiTextComponent);

impl Deref for TextComponent {
    type Target = UiTextComponent;

    fn deref(&self) -> &UiTextComponent {
        &self.0
    }
}

impl DerefMut for TextComponent {
    fn deref_mut(&mut self) -> &mut UiTextComponent {
        &mut self.0
    }
}

impl SceneComponent for TextComponent {
    const TYPE_NAME: &'static str = "sindri.text";
}
