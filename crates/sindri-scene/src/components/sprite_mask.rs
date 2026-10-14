//! `sindri.sprite_mask`: a picture whose shape decides where sprites draw.

use serde::{Deserialize, Serialize};
use sindri_core::{SceneComponent, SpriteRef, SpriteRefError};

/// A mask placed in the world like a sprite but never drawn: the opaque part
/// of its picture is where sprites masked `inside` it show, and the clear
/// part is where those masked `outside` it show.
///
/// Sprites find it by `name` rather than by being its children, because what
/// a keyhole reveals stays where it is while the keyhole moves.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpriteMaskComponent {
    /// The picture whose alpha is the shape, as a sprite names one.
    pub texture: String,
    /// What a masked sprite's `mask_name` matches; empty is a name too.
    pub name: String,
}

impl SpriteMaskComponent {
    /// The picture and which part of it.
    ///
    /// # Errors
    /// When the reference does not parse.
    pub fn reference(&self) -> Result<SpriteRef, SpriteRefError> {
        SpriteRef::parse(&self.texture)
    }
}

impl SceneComponent for SpriteMaskComponent {
    const TYPE_NAME: &'static str = "sindri.sprite_mask";
}

/// How a sprite meets the mask it names.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MaskInteraction {
    /// It ignores masks.
    #[default]
    None,
    /// It shows only where the mask is opaque.
    Inside,
    /// It shows only where the mask is not.
    Outside,
}

impl MaskInteraction {
    pub const NAMES: [&'static str; 3] = ["none", "inside", "outside"];
}
