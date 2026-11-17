//! What a scene refuses, and why.

use thiserror::Error;

use crate::SceneMigrationError;

use super::document::SceneEntityId;

#[derive(Clone, Debug, Error, PartialEq)]
pub enum SceneError {
    #[error("scene entity IDs cannot be empty")]
    EmptyEntityId,
    #[error("scene contains duplicate entity IDs")]
    DuplicateEntityId,
    #[error("scene format {found} is unsupported; this runtime supports {supported}")]
    UnsupportedVersion { found: u32, supported: u32 },
    #[error("entity {entity:?} refers to missing parent {parent:?}")]
    MissingParent {
        entity: SceneEntityId,
        parent: SceneEntityId,
    },
    #[error("hierarchy cycle detected at entity {0:?}")]
    HierarchyCycle(SceneEntityId),
    #[error("entity {0:?} has a transform containing a non-finite value")]
    NonFiniteTransform(SceneEntityId),
    #[error(
        "entity {0:?} is a prefab instance and carries components of its own; an \
         instance's components are its prefab's, changed through its overrides"
    )]
    InstanceComponents(SceneEntityId),
}

impl SceneError {
    /// The error's stable name, which tools match on; unlike the message, it
    /// is never reworded.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::EmptyEntityId => "SCENE_EMPTY_ENTITY_ID",
            Self::DuplicateEntityId => "SCENE_DUPLICATE_ENTITY_ID",
            Self::UnsupportedVersion { .. } => "SCENE_UNSUPPORTED_VERSION",
            Self::MissingParent { .. } => "SCENE_MISSING_PARENT",
            Self::HierarchyCycle(_) => "SCENE_HIERARCHY_CYCLE",
            Self::NonFiniteTransform(_) => "SCENE_NON_FINITE_TRANSFORM",
            Self::InstanceComponents(_) => "SCENE_INSTANCE_COMPONENTS",
        }
    }
}

/// Failures raised while reading or writing serialized scenes.
///
/// [`SceneError`] stays comparable and free of I/O concerns; this type carries
/// the JSON and migration failures that surround it.
#[derive(Debug, Error)]
pub enum SceneJsonError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Invalid(#[from] SceneError),
    #[error(transparent)]
    Migration(#[from] SceneMigrationError),
}

impl SceneJsonError {
    /// The error's stable name: a scene's own, or that it did not read.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Json(_) => "SCENE_JSON",
            Self::Invalid(error) => error.code(),
            Self::Migration(_) => "SCENE_MIGRATION",
        }
    }
}
