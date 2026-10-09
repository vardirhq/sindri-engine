//! What can go wrong registering, describing or checking a component.

use thiserror::Error;

use crate::{SceneEntityId, SceneError};

#[derive(Debug, Error)]
pub enum ComponentRegistryError {
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
