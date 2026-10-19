//! Reusable render factors resolved at the scene seam, without GPU ownership.

use std::collections::{BTreeMap, BTreeSet};

use sindri_core::{AssetId, ComponentSchemaRegistry, ProfileDocument, SceneComponent, World};
use sindri_render::{MaterialFactors, MaterialSidedness};
use thiserror::Error;

use crate::MaterialComponent;

#[derive(Debug, Error)]
pub enum RenderMaterialError {
    #[error(transparent)]
    Factors(#[from] sindri_render::MaterialError),
    #[error("render material profile '{id}': {reason}")]
    Invalid { id: String, reason: String },
    #[error("render material profile '{0}' is not loaded or has a different type")]
    Missing(String),
}

pub(crate) fn check_reference(id: &str) -> Result<(), RenderMaterialError> {
    if id.is_empty() {
        return Ok(());
    }
    AssetId::new(id).map_err(|error| RenderMaterialError::Invalid {
        id: id.into(),
        reason: error.to_string(),
    })?;
    if !id
        .to_ascii_lowercase()
        .ends_with(sindri_core::PROFILE_SUFFIX)
    {
        return Err(RenderMaterialError::Invalid {
            id: id.into(),
            reason: "expected a .profile asset".into(),
        });
    }
    Ok(())
}

/// Asset-free values copied into each frame; old frames retain their selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedRenderMaterial {
    pub factors: MaterialFactors,
    pub sidedness: MaterialSidedness,
}

/// Checks engine-owned `render_material` values; other profile types are untouched.
/// Texture and alpha ownership remain with the geometry in this material slice.
///
/// # Errors
/// Returns an asset-named error for an invalid field, factor, version or policy.
pub fn render_material_profile(
    id: &str,
    profile: &ProfileDocument,
) -> Result<Option<ResolvedRenderMaterial>, RenderMaterialError> {
    if profile.profile_type != "render_material" {
        return Ok(None);
    }
    let bad = |reason: String| RenderMaterialError::Invalid {
        id: id.into(),
        reason,
    };
    profile.validate().map_err(|error| bad(error.to_string()))?;
    for key in profile.values.keys() {
        if !matches!(
            key.as_str(),
            "base_color" | "metallic" | "roughness" | "emissive" | "sidedness"
        ) {
            return Err(bad(format!("unknown material field '{key}'")));
        }
    }
    let value = serde_json::Value::Object(profile.values.clone().into_iter().collect());
    let component: MaterialComponent =
        serde_json::from_value(value).map_err(|error| bad(error.to_string()))?;
    Ok(Some(ResolvedRenderMaterial {
        factors: component.factors,
        sidedness: component.sidedness,
    }))
}

/// A complete, validated replacement built from asynchronously loaded profiles.
#[derive(Clone, Debug, Default)]
pub struct RenderMaterialSources {
    materials: BTreeMap<String, ResolvedRenderMaterial>,
}

impl RenderMaterialSources {
    /// # Errors
    /// Returns the invalid asset's identity without modifying the previous set.
    pub fn from_profiles<'a>(
        profiles: impl IntoIterator<Item = (&'a str, &'a ProfileDocument)>,
    ) -> Result<Self, RenderMaterialError> {
        let mut materials = BTreeMap::new();
        for (id, profile) in profiles {
            if let Some(material) = render_material_profile(id, profile)? {
                check_reference(id)?;
                materials.insert(id.into(), material);
            }
        }
        Ok(Self { materials })
    }

    /// Resolves a profile first, then explicit local overrides. Without a profile,
    /// existing inline materials keep their exact factors and visibility policy.
    ///
    /// # Errors
    /// Returns a named error for a missing profile or a profile of another type.
    pub fn resolve(
        &self,
        component: &MaterialComponent,
    ) -> Result<ResolvedRenderMaterial, RenderMaterialError> {
        let local = ResolvedRenderMaterial {
            factors: component.factors,
            sidedness: component.sidedness,
        };
        if component.profile.is_empty() {
            return Ok(local);
        }
        let mut material = *self
            .materials
            .get(&component.profile)
            .ok_or_else(|| RenderMaterialError::Missing(component.profile.clone()))?;
        if component.override_factors {
            material.factors = local.factors;
        }
        if component.override_sidedness {
            material.sidedness = local.sidedness;
        }
        Ok(material)
    }
}

/// References on all entities, including inactive ones and expanded prefabs.
///
/// # Errors
/// Returns a malformed component's registry diagnostic.
pub fn referenced_render_materials(
    world: &World,
    components: &ComponentSchemaRegistry,
) -> Result<BTreeSet<String>, sindri_core::ComponentRegistryError> {
    let mut found = BTreeSet::new();
    for (_, data) in world.entities() {
        if let Some(payload) = data.components.get(MaterialComponent::TYPE_NAME) {
            let material = components.decode::<MaterialComponent>(payload)?;
            if !material.profile.is_empty() {
                found.insert(material.profile);
            }
        }
    }
    Ok(found)
}

/// A complete editable starter for the engine-owned render material profile type.
#[must_use]
pub fn render_material_template() -> ProfileDocument {
    ProfileDocument {
        name: "New Render Material".into(),
        profile_type: "render_material".into(),
        values: [
            ("base_color".into(), serde_json::json!([1.0, 1.0, 1.0, 1.0])),
            ("metallic".into(), serde_json::json!(0.0)),
            ("roughness".into(), serde_json::json!(1.0)),
            ("emissive".into(), serde_json::json!([0.0, 0.0, 0.0])),
            ("sidedness".into(), serde_json::json!("inherit")),
        ]
        .into(),
        ..ProfileDocument::default()
    }
}

#[cfg(test)]
mod tests;
