//! Authored geometry policy; gameplay supplies world-space displacement.
use serde::{Deserialize, Serialize};
use sindri_core::SceneComponent;
use sindri_physics::{CharacterOptions3d, PhysicsError};

/// Settings for a scene-owned stationary kinematic character.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CharacterOptions3d", into = "CharacterOptions3d")]
pub struct Character3dComponent(pub CharacterOptions3d);

impl TryFrom<CharacterOptions3d> for Character3dComponent {
    type Error = PhysicsError;

    fn try_from(options: CharacterOptions3d) -> Result<Self, Self::Error> {
        options.validate()?;
        Ok(Self(options))
    }
}

impl From<Character3dComponent> for CharacterOptions3d {
    fn from(component: Character3dComponent) -> Self {
        component.0
    }
}

impl SceneComponent for Character3dComponent {
    const TYPE_NAME: &'static str = "sindri.physics3d.character";
}
