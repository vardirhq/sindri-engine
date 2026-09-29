//! What a scene file holds, and what it refuses to hold.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{SceneMigrator, Transform3D};

use super::canonical::collapse_scalar_arrays;
use super::error::{SceneError, SceneJsonError};
use super::graph::validate_entities;

pub const SCENE_FORMAT_VERSION: u32 = 10;
pub const SCENE_SUFFIX: &str = ".scene";
pub const LEGACY_SCENE_SUFFIX: &str = ".scene.json";

/// A stable, project-authored entity identifier used only in serialized data.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SceneEntityId(String);

impl SceneEntityId {
    pub fn new(value: impl Into<String>) -> Result<Self, SceneError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(SceneError::EmptyEntityId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SceneMetadata {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub editor: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneEntity {
    pub id: SceneEntityId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<SceneEntityId>,
    #[serde(default)]
    pub transform: Transform3D,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub components: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub editor: BTreeMap<String, Value>,
}

impl SceneEntity {
    #[must_use]
    pub fn new(id: SceneEntityId) -> Self {
        Self { id, parent: None, transform: Transform3D::default(), components: BTreeMap::new(), editor: BTreeMap::new() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneDocument {
    pub format_version: u32,
    #[serde(default)]
    pub metadata: SceneMetadata,
    #[serde(default)]
    pub entities: Vec<SceneEntity>,
}

impl Default for SceneDocument {
    fn default() -> Self {
        Self { format_version: SCENE_FORMAT_VERSION, metadata: SceneMetadata::default(), entities: Vec::new() }
    }
}

impl SceneDocument {
    pub fn from_json(json: &str) -> Result<Self, SceneJsonError> {
        let value: Value = serde_json::from_str(json)?;
        let value = SceneMigrator::default().migrate(value)?;
        let document: Self = serde_json::from_value(value)?;
        document.validate()?;
        Ok(document)
    }

    pub fn to_canonical_json(&self) -> Result<String, SceneJsonError> {
        let canonical = self.canonicalized();
        canonical.validate()?;
        let mut json = collapse_scalar_arrays(&serde_json::to_string_pretty(&canonical)?);
        json.push('\n');
        Ok(json)
    }

    #[must_use]
    pub fn canonicalized(&self) -> Self {
        let mut canonical = self.clone();
        canonical.canonicalize();
        canonical
    }

    pub fn canonicalize(&mut self) { self.entities.sort_by(|left, right| left.id.cmp(&right.id)); }

    pub fn strip_editor_metadata(&mut self) {
        self.metadata.editor.clear();
        for entity in &mut self.entities { entity.editor.clear(); }
    }

    pub fn validate(&self) -> Result<(), SceneError> {
        if self.format_version != SCENE_FORMAT_VERSION {
            return Err(SceneError::UnsupportedVersion { found: self.format_version, supported: SCENE_FORMAT_VERSION });
        }
        validate_entities(&self.entities)
    }

    pub fn entity(&self, id: &SceneEntityId) -> Option<&SceneEntity> {
        self.entities.iter().find(|entity| &entity.id == id)
    }
}
