//! What can go wrong registering, describing or checking a component.

use thiserror::Error;

use crate::{SceneEntityId, SceneError};

#[derive(Debug, Error)]
pub enum ComponentRegistryError {
    #[error(
        "entity '{}': component key '{key}' names a part with something other than \
         letters, digits, '-' and '_'",
        entity.as_str()
    )]
    InvalidPartName { entity: SceneEntityId, key: String },
    #[error("component type name '{0}' is invalid")]
    InvalidTypeName(&'static str),
    #[error("component display names cannot be empty")]
    EmptyDisplayName,
    #[error("component type '{0}' is already registered")]
    AlreadyRegistered(String),
    #[error("component type '{0}' is not registered")]
    NotRegistered(&'static str),
    #[error("the field template for component type '{0}' is not an object")]
    InvalidFields(&'static str),
    #[error("component type '{0}' has no field template, so its fields cannot be described")]
    DescribedWithoutFields(&'static str),
    #[error("component type '{type_name}' has no field at '{path}'")]
    UnknownFieldPath {
        type_name: &'static str,
        path: String,
    },
    #[error(
        "the field at '{path}' of component type '{type_name}' is not a choice, \
         so its variants have no spellings to be picked by"
    )]
    VariantsWithoutChoice {
        type_name: &'static str,
        path: String,
    },
    #[error(
        "'{variant}' is not one of the spellings the choice at '{path}' of \
         component type '{type_name}' accepts"
    )]
    UnknownVariant {
        type_name: &'static str,
        path: String,
        variant: String,
    },
    #[error(
        "the choice at '{path}' of component type '{type_name}' accepts \
         '{variant}', which no variant describes"
    )]
    UndescribedVariant {
        type_name: &'static str,
        path: String,
        variant: String,
    },
    #[error(
        "the template for variant '{variant}' of component type '{type_name}' is not an object"
    )]
    InvalidVariantTemplate {
        type_name: &'static str,
        variant: String,
    },
    #[error(
        "choosing '{variant}' for component type '{type_name}' does not produce \
         a component the engine accepts"
    )]
    VariantMismatch {
        type_name: &'static str,
        variant: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("the field template for component type '{type_name}' does not match it: {wrong}")]
    TemplateMismatch {
        type_name: &'static str,
        wrong: String,
    },
    #[error(
        "the field at '{path}' of component type '{type_name}' is not empty in its \
         template, so it cannot be one that is absent until added"
    )]
    NotOptional {
        type_name: &'static str,
        path: String,
    },
    #[error(
        "adding the field at '{path}' to component type '{type_name}' does not \
         produce a component the engine accepts"
    )]
    OptionalMismatch {
        type_name: &'static str,
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("the payload registered for component type '{type_name}' does not decode as it")]
    InvalidDefault {
        type_name: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("entity {entity:?} contains unknown component type '{type_name}'")]
    UnknownComponent {
        entity: SceneEntityId,
        type_name: String,
    },
    #[error("entity '{entity}' has invalid '{type_name}' component data")]
    InvalidPayload {
        entity: String,
        type_name: String,
        #[source]
        source: serde_json::Error,
    },
    #[error(transparent)]
    InvalidScene(#[from] SceneError),
}

impl ComponentRegistryError {
    /// The error's stable name, which tools match on.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidPartName { .. } => "COMPONENT_INVALID_PART_NAME",
            Self::InvalidTypeName(_) => "COMPONENT_INVALID_TYPE_NAME",
            Self::EmptyDisplayName => "COMPONENT_EMPTY_DISPLAY_NAME",
            Self::AlreadyRegistered(_) => "COMPONENT_ALREADY_REGISTERED",
            Self::NotRegistered(_) => "COMPONENT_NOT_REGISTERED",
            Self::InvalidFields(_) => "COMPONENT_INVALID_FIELDS",
            Self::DescribedWithoutFields(_) => "COMPONENT_DESCRIBED_WITHOUT_FIELDS",
            Self::UnknownFieldPath { .. } => "COMPONENT_UNKNOWN_FIELD_PATH",
            Self::VariantsWithoutChoice { .. } => "COMPONENT_VARIANTS_WITHOUT_CHOICE",
            Self::UnknownVariant { .. } => "COMPONENT_UNKNOWN_VARIANT",
            Self::UndescribedVariant { .. } => "COMPONENT_UNDESCRIBED_VARIANT",
            Self::InvalidVariantTemplate { .. } => "COMPONENT_INVALID_VARIANT_TEMPLATE",
            Self::VariantMismatch { .. } => "COMPONENT_VARIANT_MISMATCH",
            Self::TemplateMismatch { .. } => "COMPONENT_TEMPLATE_MISMATCH",
            Self::NotOptional { .. } => "COMPONENT_NOT_OPTIONAL",
            Self::OptionalMismatch { .. } => "COMPONENT_OPTIONAL_MISMATCH",
            Self::InvalidDefault { .. } => "COMPONENT_INVALID_DEFAULT",
            Self::UnknownComponent { .. } => "COMPONENT_UNKNOWN",
            Self::InvalidPayload { .. } => "COMPONENT_INVALID_PAYLOAD",
            Self::InvalidScene(error) => error.code(),
        }
    }
}
