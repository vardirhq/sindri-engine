//! Typed physics profiles resolved at the scene boundary, never in the backend.

use std::collections::BTreeMap;

use serde::Deserialize;
use sindri_core::{ComponentSchemaRegistry, ProfileDocument, SceneComponent, World};
use sindri_physics::{Collider2d, PhysicsMaterial};
use thiserror::Error;

use crate::PhysicsSyncError;

/// Applies one reusable material to all an entity's collider pieces. An empty
/// profile keeps the pieces' literal coefficients; enabled overrides win last.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PhysicsMaterial2dComponent {
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub override_friction: bool,
    #[serde(default = "default_friction")]
    pub friction: f32,
    #[serde(default)]
    pub override_restitution: bool,
    #[serde(default)]
    pub restitution: f32,
}

const fn default_friction() -> f32 {
    0.5
}

impl SceneComponent for PhysicsMaterial2dComponent {
    const TYPE_NAME: &'static str = "sindri.physics2d.material";
}

#[derive(Debug, Error)]
pub enum PhysicsMaterialError {
    #[error("physics material profile '{id}': {reason}")]
    Invalid { id: String, reason: String },
    #[error("physics material profile '{0}' is not loaded or has a different type")]
    Missing(String),
}

/// Validates a physics profile with the same rules on every host. Other profile
/// types remain ordinary game data. Both coefficients are required; unknown
/// keys and non-numeric values are errors rather than silently ignored typos.
///
/// # Errors
/// Returns an asset-named error for invalid material fields or coefficients.
pub fn physics_material_profile(
    id: &str,
    profile: &ProfileDocument,
) -> Result<Option<PhysicsMaterial>, PhysicsMaterialError> {
    if profile.profile_type != "physics_material" {
        return Ok(None);
    }
    let values = serde_json::Value::Object(
        profile
            .values
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    );
    let material: PhysicsMaterial =
        serde_json::from_value(values).map_err(|error| PhysicsMaterialError::Invalid {
            id: id.into(),
            reason: error.to_string(),
        })?;
    material
        .validate()
        .map_err(|error| PhysicsMaterialError::Invalid {
            id: id.into(),
            reason: error.to_string(),
        })?;
    Ok(Some(material))
}

/// Asset-free resolved values supplied by an asynchronous host asset loader.
#[derive(Clone, Debug, Default)]
pub struct PhysicsMaterialSources {
    materials: BTreeMap<String, PhysicsMaterial>,
}

impl PhysicsMaterialSources {
    /// Builds a complete replacement, so a failed reload cannot partly update
    /// the previous valid source set.
    ///
    /// # Errors
    /// Returns the first invalid physics profile, with its asset identity.
    pub fn from_profiles<'a>(
        profiles: impl IntoIterator<Item = (&'a str, &'a ProfileDocument)>,
    ) -> Result<Self, PhysicsMaterialError> {
        let mut materials = BTreeMap::new();
        for (id, profile) in profiles {
            if let Some(material) = physics_material_profile(id, profile)? {
                materials.insert(id.into(), material);
            }
        }
        Ok(Self { materials })
    }

    pub(crate) fn apply(
        &self,
        component: &PhysicsMaterial2dComponent,
        pieces: &mut [Collider2d],
    ) -> Result<(), PhysicsMaterialError> {
        let material = if component.profile.is_empty() {
            None
        } else {
            Some(
                *self
                    .materials
                    .get(&component.profile)
                    .ok_or_else(|| PhysicsMaterialError::Missing(component.profile.clone()))?,
            )
        };
        for piece in pieces {
            if let Some(material) = material {
                piece.friction = material.friction;
                piece.restitution = material.restitution;
            }
            if component.override_friction {
                piece.friction = component.friction;
            }
            if component.override_restitution {
                piece.restitution = component.restitution;
            }
            PhysicsMaterial {
                friction: piece.friction,
                restitution: piece.restitution,
            }
            .validate()
            .map_err(|error| PhysicsMaterialError::Invalid {
                id: component.profile.clone(),
                reason: error.to_string(),
            })?;
        }
        Ok(())
    }
}

/// Profile references on scene and prefab entities, collected for export even
/// when the entity starts inactive. Runtime-spawned prefabs use the same walker.
///
/// # Errors
/// Returns an error for malformed material components.
pub fn referenced_physics_materials(
    world: &World,
    components: &ComponentSchemaRegistry,
) -> Result<Vec<String>, PhysicsSyncError> {
    let mut profiles = Vec::new();
    for (_, data) in world.entities() {
        if let Some(payload) = data.components.get(PhysicsMaterial2dComponent::TYPE_NAME) {
            let material = components.decode::<PhysicsMaterial2dComponent>(payload)?;
            if !material.profile.is_empty() {
                profiles.push(material.profile);
            }
        }
    }
    Ok(profiles)
}

#[cfg(test)]
mod tests;
