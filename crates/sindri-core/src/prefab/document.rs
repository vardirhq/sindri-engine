//! What a prefab file holds, and what it refuses to hold.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::scene::{collapse_scalar_arrays, roots, validate_entities};
use crate::{SceneEntity, SceneEntityId, SceneError, SceneMetadata, SceneMigrationError};

/// The prefab format's own version, which is not the scene's.
pub const PREFAB_FORMAT_VERSION: u32 = 1;
/// Native suffix for prefab assets.
pub const PREFAB_SUFFIX: &str = ".prefab";
/// Legacy suffix accepted while projects migrate to the native extension.
pub const LEGACY_PREFAB_SUFFIX: &str = ".prefab.json";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrefabDocument {
    pub format_version: u32,
    #[serde(default)]
    pub metadata: SceneMetadata,
    #[serde(default)]
    pub entities: Vec<SceneEntity>,
}

impl Default for PrefabDocument {
    fn default() -> Self {
        Self {
            format_version: PREFAB_FORMAT_VERSION,
            metadata: SceneMetadata::default(),
            entities: Vec::new(),
        }
    }
}

impl PrefabDocument {
    pub fn single(entity: SceneEntity) -> Self {
        Self { entities: vec![entity], ..Self::default() }
    }

    pub fn from_json(json: &str) -> Result<Self, PrefabJsonError> {
        let document: Self = serde_json::from_str(json)?;
        document.validate()?;
        Ok(document)
    }

    pub fn to_canonical_json(&self) -> Result<String, PrefabJsonError> {
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

    pub fn canonicalize(&mut self) {
        self.entities.sort_by(|left, right| left.id.cmp(&right.id));
    }

    pub fn strip_editor_metadata(&mut self) {
        self.metadata.editor.clear();
        for entity in &mut self.entities { entity.editor.clear(); }
    }

    pub fn entity(&self, id: &SceneEntityId) -> Option<&SceneEntity> {
        self.entities.iter().find(|entity| &entity.id == id)
    }

    pub fn root(&self) -> Result<&SceneEntity, PrefabError> {
        let mut found = roots(&self.entities);
        let root = found.next().ok_or(PrefabError::NoRoot)?;
        match found.next() {
            None => Ok(root),
            Some(_) => Err(PrefabError::SeveralRoots(roots(&self.entities).count())),
        }
    }

    pub fn children_of(&self, parent: &SceneEntityId) -> impl Iterator<Item = &SceneEntity> {
        self.entities.iter().filter(move |entity| entity.parent.as_ref() == Some(parent))
    }

    pub fn validate(&self) -> Result<(), PrefabError> {
        if self.format_version != PREFAB_FORMAT_VERSION {
            return Err(PrefabError::UnsupportedVersion { found: self.format_version, supported: PREFAB_FORMAT_VERSION });
        }
        if self.entities.is_empty() { return Err(PrefabError::NoRoot); }
        validate_entities(&self.entities)?;
        let count = roots(&self.entities).count();
        match count {
            1 => Ok(()),
            0 => Err(PrefabError::NoRoot),
            _ => Err(PrefabError::SeveralRoots(count)),
        }
    }

    pub fn component_names(&self) -> impl Iterator<Item = &str> {
        self.entities.iter().flat_map(|entity| entity.components.keys().map(String::as_str))
    }
}

impl FromIterator<(String, Value)> for PrefabDocument {
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(components: T) -> Self {
        let mut entity = SceneEntity::new(SceneEntityId::new("root").expect("a non-empty literal is a valid identity"));
        entity.components = components.into_iter().collect::<BTreeMap<_, _>>();
        Self::single(entity)
    }
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum PrefabError {
    #[error("prefab format {found} is unsupported; this runtime supports {supported}")]
    UnsupportedVersion { found: u32, supported: u32 },
    #[error("a prefab needs exactly one root entity, and this one has none")]
    NoRoot,
    #[error("a prefab needs exactly one root entity, and this one has {0}")]
    SeveralRoots(usize),
    #[error(transparent)]
    Entities(#[from] SceneError),
}

#[derive(Debug, Error)]
pub enum PrefabJsonError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Invalid(#[from] PrefabError),
    #[error(transparent)]
    Migration(#[from] SceneMigrationError),
}
