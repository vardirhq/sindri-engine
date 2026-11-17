//! JSON Schemas for `.scene` and `.prefab` files, from the component registry.
//!
//! What shape is a scene file? Until now the answer was "open Rust": the
//! document, entity and transform types in `sindri-core`, and the component
//! payloads in whichever crate registered them. An agent writing a scene by
//! hand guessed, and a person editing one in an editor got no completion.
//!
//! The registry already knows each payload's shape — its field template is
//! the component at rest — and what each field means. This writes that down
//! as JSON Schema (draft 2020-12), so the same file gives an agent the shape
//! and gives VS Code validation and completion.
//!
//! What the schema can promise, and what it leaves to `sindri project check`:
//!
//! - A component's **own** fields are strict: a misspelled field is an error,
//!   because serde would otherwise ignore it and the scene would quietly do
//!   less than its author meant.
//! - A field **inside** a component's object is typed but not closed, because
//!   a template cannot tell a struct from a map keyed by name (a sequence's
//!   `sequences` is the case), and refusing every second key of a map would be
//!   a schema that lies.
//! - A choice with variants is a `oneOf` over them, each with its own fields.
//! - References — that a parent exists, that an asset is on disk, that a
//!   script declares what it is named for — are not shapes, so they are the
//!   check's.

use serde_json::{Map, Value, json};
use sindri_core::{
    ComponentSchemaRegistry, FieldMeaning, PART_SEPARATOR, PREFAB_FORMAT_VERSION,
    SCENE_FORMAT_VERSION,
};
use sindri_scene::SceneExtractor;

use crate::CapabilitiesError;

/// Components a scene may carry that the scene registry does not describe,
/// because another layer owns them.
fn unregistered() -> Vec<(&'static str, Value)> {
    vec![
        (
            // `sindri_decay::ScriptComponent`, registered by the Decay host.
            "sindri.script",
            json!({
                "title": "Script",
                "type": "object",
                "required": ["source", "script"],
                "additionalProperties": false,
                "properties": {
                    "source": {
                        "type": "string",
                        "minLength": 1,
                        "description": "The Decay file, relative to the project. sindri.toml must list it under scripts."
                    },
                    "script": {
                        "type": "string",
                        "minLength": 1,
                        "description": "The script container in that file this entity runs."
                    },
                    "properties": {
                        "type": "object",
                        "description": "Values for the script's declared properties, by name."
                    },
                    "enabled": { "type": "boolean", "default": true }
                }
            }),
        ),
        (
            "weave.style",
            json!({
                "description": "Weave styling for this entity: its classes, id and inline declarations, read by sindri-weave."
            }),
        ),
    ]
}

/// Components that also accept one piece written on its own, the form every
/// project used before compounds: `{ "shape": … }` as well as
/// `{ "pieces": [ … ] }`. The engine reads both, so the schema does.
const SINGLE_PIECE: &[&str] = &["sindri.physics2d.collider", "sindri.physics3d.collider"];

/// Fields a component's renderer reads beside its typed ones, so its template
/// cannot show them.
fn beside(type_name: &str) -> Option<(&'static str, Value)> {
    (type_name == "sindri.shape").then(|| {
        (
            "points",
            json!({
                "type": "array",
                "items": {
                    "type": "array",
                    "items": { "type": "number" },
                    "minItems": 2,
                    "maxItems": 2
                },
                "description": "A polygon's own corners, x and y, instead of a regular one's."
            }),
        )
    })
}

/// Which document a schema describes.
#[derive(Clone, Copy)]
pub(crate) enum Document {
    Scene,
    Prefab,
}

/// The schema for one kind of document.
pub(crate) fn describe(document: Document) -> Result<Value, CapabilitiesError> {
    let extractor = SceneExtractor::new()?;
    let registry = extractor.components();
    let (title, about, version, migrates) = match document {
        Document::Scene => (
            "Sindri scene",
            "A Sindri `.scene` file: a flat list of entities, each with an id, an optional parent, a transform and components keyed by type name. Older format versions are migrated when opened.",
            SCENE_FORMAT_VERSION,
            true,
        ),
        Document::Prefab => (
            "Sindri prefab",
            "A Sindri `.prefab` file: a reusable entity tree, written like a scene's entities. The first entity without a parent is the root.",
            PREFAB_FORMAT_VERSION,
            false,
        ),
    };
    let format_version = if migrates {
        json!({
            "type": "integer",
            "minimum": 1,
            "maximum": version,
            "default": version,
            "description": format!("The format this file is written in. {version} is current; an older one is migrated when the file is opened."),
        })
    } else {
        json!({
            "const": version,
            "description": format!("The format this file is written in; {version} is the only one."),
        })
    };

    let mut definitions = Map::new();
    definitions.insert("entity".to_owned(), entity());
    definitions.insert("transform".to_owned(), transform());
    definitions.insert("prefab_instance".to_owned(), prefab_instance());
    definitions.insert("components".to_owned(), components(registry, false));
    definitions.insert("component_overrides".to_owned(), components(registry, true));
    definitions.insert(
        "editor".to_owned(),
        json!({
            "type": "object",
            "description": "Editor-only state. Runtimes ignore it, and a tool may drop it.",
        }),
    );
    let mut payloads: Vec<(String, Value)> = registry
        .registered_components()
        .map(|metadata| {
            let type_name = metadata.type_name.as_str();
            (
                type_name.to_owned(),
                payload(registry, type_name, &metadata.display_name),
            )
        })
        .collect();
    payloads.sort_by(|left, right| left.0.cmp(&right.0));
    for (type_name, schema) in payloads {
        definitions.insert(type_name, schema);
    }
    for (type_name, schema) in unregistered() {
        definitions.insert(type_name.to_owned(), schema);
    }

    Ok(json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": title,
        "description": format!("{about} Generated by `{}` from the component registry; edits here are overwritten.", crate::REGENERATE_COMMAND),
        "type": "object",
        "required": ["format_version"],
        "additionalProperties": false,
        "properties": {
            "format_version": format_version,
            "metadata": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "name": { "type": "string" },
                    "editor": { "$ref": "#/$defs/editor" }
                }
            },
            "entities": {
                "type": "array",
                "items": { "$ref": "#/$defs/entity" }
            }
        },
        "$defs": definitions,
    }))
}

fn entity() -> Value {
    json!({
        "type": "object",
        "required": ["id"],
        "additionalProperties": false,
        "properties": {
            "id": {
                "type": "string",
                "minLength": 1,
                "description": "A stable id, unique in this file. Parents, scripts and prefab overrides name an entity by it."
            },
            "name": { "type": ["string", "null"], "description": "What a person calls it. Not unique." },
            "parent": {
                "type": ["string", "null"],
                "minLength": 1,
                "description": "The id of the entity this one sits under; its transform is relative to that one's."
            },
            "transform_3d": { "$ref": "#/$defs/transform" },
            "components": { "$ref": "#/$defs/components" },
            "disabled": {
                "type": "boolean",
                "description": "Switched off: it, and everything under it, takes no part in the scene."
            },
            "editor": { "$ref": "#/$defs/editor" },
            "prefab": { "$ref": "#/$defs/prefab_instance" }
        }
    })
}

fn transform() -> Value {
    let numbers = |count: usize, about: &str| {
        json!({
            "type": "array",
            "items": { "type": "number" },
            "minItems": count,
            "maxItems": count,
            "description": about,
        })
    };
    json!({
        "type": "object",
        "additionalProperties": false,
        "description": "Where the entity is, relative to its parent. Any field left out is the identity's.",
        "properties": {
            "position": numbers(3, "x, y, z. In 2D, z is the layer."),
            "rotation": numbers(4, "A quaternion, in x, y, z, w order."),
            "scale": numbers(3, "x, y, z."),
            "z_locked": {
                "type": "boolean",
                "description": "Declares that this entity stays on the layer it is on."
            }
        }
    })
}

fn prefab_instance() -> Value {
    json!({
        "type": "object",
        "required": ["source"],
        "additionalProperties": false,
        "description": "This entity is an instance of a prefab. It carries no components of its own: what it is made of is the prefab's, changed by its overrides.",
        "properties": {
            "source": {
                "type": "string",
                "minLength": 1,
                "description": "The prefab file, relative to the project."
            },
            "overrides": {
                "type": "object",
                "description": "Changes to the prefab's entities, keyed by their id in the prefab.",
                "additionalProperties": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "name": { "type": "string" },
                        "transform_3d": { "$ref": "#/$defs/transform" },
                        "disabled": { "type": "boolean" },
                        "components": { "$ref": "#/$defs/component_overrides" },
                        "editor": { "$ref": "#/$defs/editor" }
                    }
                }
            },
            "removed": {
                "type": "array",
                "items": { "type": "string" },
                "uniqueItems": true,
                "description": "Ids of the prefab's entities this instance leaves out."
            }
        }
    })
}

/// An entity's `components`: each registered type by name, and each named
/// part of it (`type#name`). An override's components are merge patches, so
/// they name a known type but are not otherwise shaped.
fn components(registry: &ComponentSchemaRegistry, overrides: bool) -> Value {
    let mut names: Vec<&str> = registry
        .registered_components()
        .map(|metadata| metadata.type_name.as_str())
        .chain(unregistered().into_iter().map(|(name, _)| name))
        .collect();
    names.sort_unstable();
    let mut properties = Map::new();
    let mut patterns = Map::new();
    for name in names {
        let schema = if overrides {
            json!({
                "type": ["object", "null"],
                "description": "A merge patch over the prefab's payload: a field replaces, null removes."
            })
        } else {
            json!({ "$ref": format!("#/$defs/{}", pointer_escape(name)) })
        };
        patterns.insert(
            format!("^{}{PART_SEPARATOR}[A-Za-z0-9_-]+$", regex_escape(name)),
            schema.clone(),
        );
        properties.insert(name.to_owned(), schema);
    }
    json!({
        "type": "object",
        "description": "Components keyed by type name. A second of a kind is a named part: the type, `#`, and the part's name, such as `sindri.light2d#halo`.",
        "properties": properties,
        "patternProperties": patterns,
        "additionalProperties": false,
    })
}

/// One component's payload.
fn payload(registry: &ComponentSchemaRegistry, type_name: &str, display_name: &str) -> Value {
    let template = registry
        .fields(type_name)
        .or_else(|| registry.default_payload(type_name));
    let mut schema = match template {
        Some(template) => shape(registry, type_name, template, ""),
        None => json!({ "type": "object" }),
    };
    if let Some((field, extra)) = beside(type_name) {
        schema["properties"][field] = extra;
    }
    if SINGLE_PIECE.contains(&type_name) {
        let mut single = schema["properties"]["pieces"]["items"].clone();
        single["additionalProperties"] = json!(false);
        // What the one piece cannot do without, asked of the engine as the
        // component's own fields are.
        let piece = template
            .and_then(|template| template.get("pieces"))
            .and_then(Value::as_array)
            .and_then(|pieces| pieces.first())
            .and_then(Value::as_object);
        if let Some(piece) = piece {
            let decodes = |payload: &Map<String, Value>| {
                registry
                    .validate_payload(type_name, &Value::Object(payload.clone()))
                    .is_ok()
            };
            if decodes(piece) {
                let required: Vec<&String> = piece
                    .keys()
                    .filter(|name| {
                        let mut without = piece.clone();
                        without.remove(*name);
                        !decodes(&without)
                    })
                    .collect();
                if !required.is_empty() {
                    single["required"] = json!(required);
                    schema["properties"]["pieces"]["items"]["required"] = json!(required);
                }
            }
        }
        schema = json!({ "anyOf": [schema, single] });
    }
    if let Value::Object(object) = &mut schema {
        object.insert("title".to_owned(), json!(display_name));
    }
    schema
}

/// The schema of the value at `path` in a component, from its exemplar and
/// what the registry says the field means.
fn shape(
    registry: &ComponentSchemaRegistry,
    type_name: &str,
    exemplar: &Value,
    path: &str,
) -> Value {
    let mut schema = match exemplar {
        Value::Null => match registry.optional(type_name, path) {
            Some(added) => json!({
                "anyOf": [{ "type": "null" }, shape(registry, type_name, added, path)],
                "default": null,
            }),
            None => json!({}),
        },
        Value::Bool(_) => json!({ "type": "boolean" }),
        Value::Number(number) if number.is_f64() => json!({ "type": "number" }),
        Value::Number(_) => json!({ "type": "integer" }),
        Value::String(_) => json!({ "type": "string" }),
        Value::Array(items) => match items.first() {
            Some(item) => json!({
                "type": "array",
                "items": shape(registry, type_name, item, &format!("{path}[]")),
            }),
            None => json!({ "type": "array" }),
        },
        Value::Object(fields) => object(registry, type_name, fields, path),
    };
    if !path.is_empty() && !exemplar.is_null() {
        if let Some(meaning) = registry.meaning(type_name, path) {
            annotate(&mut schema, meaning);
        }
        if let Value::Object(object) = &mut schema {
            object.insert("default".to_owned(), exemplar.clone());
        }
    }
    schema
}

/// An object, closed at the top of a component and open below it; a `oneOf`
/// when one of its fields is a choice with variants.
fn object(
    registry: &ComponentSchemaRegistry,
    type_name: &str,
    fields: &Map<String, Value>,
    path: &str,
) -> Value {
    let field_path = |name: &str| {
        if path.is_empty() {
            name.to_owned()
        } else {
            format!("{path}.{name}")
        }
    };
    let closed = path.is_empty();
    let plain = |fields: &Map<String, Value>| {
        let properties: Map<String, Value> = fields
            .iter()
            .map(|(name, value)| {
                (
                    name.clone(),
                    shape(registry, type_name, value, &field_path(name)),
                )
            })
            .collect();
        let mut schema = json!({ "type": "object", "properties": properties });
        if closed {
            schema["additionalProperties"] = json!(false);
        }
        schema
    };
    // At the top, the engine says which fields it cannot do without: the
    // whole as written decodes, and the whole without one field does not.
    let needed = |whole: &Map<String, Value>| -> Vec<String> {
        let decodes = |payload: Map<String, Value>| {
            registry
                .validate_payload(type_name, &Value::Object(payload))
                .is_ok()
        };
        if !closed || !decodes(whole.clone()) {
            return Vec::new();
        }
        whole
            .keys()
            .filter(|name| {
                let mut without = whole.clone();
                without.remove(*name);
                !decodes(without)
            })
            .cloned()
            .collect()
    };
    let tagged = fields.keys().find_map(|name| {
        registry
            .variants(type_name, &field_path(name))
            .map(|variants| (name, variants))
    });
    let Some((tag, variants)) = tagged else {
        let mut schema = plain(fields);
        let required = needed(fields);
        if !required.is_empty() {
            schema["required"] = json!(required);
        }
        return schema;
    };
    let options = variants
        .iter()
        .map(|(word, template)| {
            let mut whole = template.as_object().cloned().unwrap_or_default();
            whole.insert(tag.clone(), json!(word));
            let mut required = needed(&whole);
            if !required.contains(tag) {
                required.insert(0, tag.clone());
            }
            let mut fields = whole;
            fields.remove(tag);
            let mut schema = plain(&fields);
            schema["properties"][tag] = json!({ "const": word });
            schema["required"] = json!(required);
            // A variant names every field its spelling has, so a field it
            // does not name is a mistake: a box given a radius.
            schema["additionalProperties"] = json!(false);
            schema
        })
        .collect::<Vec<_>>();
    json!({ "type": "object", "oneOf": options })
}

/// What a field's meaning adds to its shape.
fn annotate(schema: &mut Value, meaning: &FieldMeaning) {
    let about = match meaning {
        FieldMeaning::Asset(kind) => format!(
            "Names a {} asset, by its path in the project.",
            kind.as_str().replace('_', " ")
        ),
        FieldMeaning::Choice(options) => {
            schema["enum"] = json!(options);
            "One of the listed spellings.".to_owned()
        }
        FieldMeaning::Color => {
            "A color: red, green, blue and, where there are four, alpha, each 0 to 1.".to_owned()
        }
        FieldMeaning::Angle => "An angle, in radians.".to_owned(),
        FieldMeaning::Range { min, max } => {
            if min.is_finite() {
                schema["minimum"] = json!(min);
            }
            if max.is_finite() {
                schema["maximum"] = json!(max);
            }
            "A bounded number.".to_owned()
        }
        FieldMeaning::OneOf(values) => {
            schema["enum"] = json!(values);
            "One of the listed values.".to_owned()
        }
        FieldMeaning::Mask => "A collision mask: one bit a layer.".to_owned(),
        FieldMeaning::Entity => "Names another entity, by its id.".to_owned(),
        FieldMeaning::Key => {
            "This item's key: unique in its list, named by other fields.".to_owned()
        }
        FieldMeaning::KeyOf(target) => format!("Names an item by its `{target}`."),
        FieldMeaning::Block { set, or_key_of } => {
            schema["type"] = json!(["string", "integer"]);
            format!(
                "Names a block of the tile set `{set}` names, or, without one, an item by its `{or_key_of}`."
            )
        }
    };
    if let Value::Object(object) = schema {
        object.insert("description".to_owned(), json!(about));
    }
}

/// A type name as one step of a JSON pointer.
fn pointer_escape(name: &str) -> String {
    name.replace('~', "~0").replace('/', "~1")
}

/// A type name, matched literally in a pattern.
fn regex_escape(name: &str) -> String {
    let mut escaped = String::with_capacity(name.len());
    for character in name.chars() {
        if "\\.+*?()|[]{}^$".contains(character) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

#[cfg(test)]
mod tests;
