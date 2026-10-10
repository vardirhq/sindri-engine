//! Authored geometry policy; gameplay supplies world-space displacement.
use serde::{Deserialize, Serialize};
use sindri_core::SceneComponent;
use sindri_physics::{CharacterOptions3d, PhysicsError};

/// Settings for a scene-owned stationary kinematic character.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "AuthoredCharacter3d", into = "AuthoredCharacter3d")]
pub struct Character3dComponent {
    pub movement: CharacterOptions3d,
    pub carry_platforms: bool,
}

impl Default for Character3dComponent {
    fn default() -> Self {
        Self {
            movement: CharacterOptions3d::default(),
            carry_platforms: true,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct AuthoredCharacter3d {
    #[serde(flatten)]
    movement: CharacterOptions3d,
    carry_platforms: bool,
}

impl Default for AuthoredCharacter3d {
    fn default() -> Self {
        Character3dComponent::default().into()
    }
}

impl TryFrom<AuthoredCharacter3d> for Character3dComponent {
    type Error = PhysicsError;
    fn try_from(authored: AuthoredCharacter3d) -> Result<Self, Self::Error> {
        authored.movement.validate()?;
        Ok(Self {
            movement: authored.movement,
            carry_platforms: authored.carry_platforms,
        })
    }
}

impl From<Character3dComponent> for AuthoredCharacter3d {
    fn from(component: Character3dComponent) -> Self {
        Self {
            movement: component.movement,
            carry_platforms: component.carry_platforms,
        }
    }
}

impl SceneComponent for Character3dComponent {
    const TYPE_NAME: &'static str = "sindri.physics3d.character";
}
