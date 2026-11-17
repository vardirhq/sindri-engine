//! The schemas accept every scene and prefab in the repository and refuse the
//! mistakes they exist to catch.
//!
//! Checked by a small validator for exactly the keywords the schemas use, so
//! the test reads the schema as an editor would rather than trusting the
//! generator's intent. A keyword it does not know fails the test, so the
//! generator cannot start relying on one this does not check.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{Document, describe};

/// Every problem `instance` has under `schema`, as `path: what`.
///
/// One match over every keyword, long because the keyword list is.
#[allow(clippy::too_many_lines)]
fn problems(root: &Value, schema: &Value, instance: &Value, at: &str, out: &mut Vec<String>) {
    let Some(schema) = schema.as_object() else {
        if schema == &Value::Bool(false) {
            out.push(format!("{at}: nothing is allowed here"));
        }
        return;
    };
    for (keyword, rule) in schema {
        match keyword.as_str() {
            "$ref" => {
                let pointer = rule
                    .as_str()
                    .and_then(|text| text.strip_prefix('#'))
                    .unwrap();
                let target = root
                    .pointer(pointer)
                    .unwrap_or_else(|| panic!("{pointer} resolves"));
                problems(root, target, instance, at, out);
            }
            "type" => {
                let types: Vec<&str> = match rule {
                    Value::String(one) => vec![one.as_str()],
                    many => many
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter_map(Value::as_str)
                        .collect(),
                };
                if !types.iter().any(|kind| is(instance, kind)) {
                    out.push(format!("{at}: {instance} is not {types:?}"));
                }
            }
            "enum"
                if !rule
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|option| same(option, instance)) =>
            {
                out.push(format!("{at}: {instance} is not one of {rule}"));
            }
            "const" if !same(rule, instance) => {
                out.push(format!("{at}: {instance} is not {rule}"));
            }
            "minimum" | "maximum" => {
                if let (Some(bound), Some(value)) = (rule.as_f64(), instance.as_f64()) {
                    let ok = if keyword == "minimum" {
                        value >= bound
                    } else {
                        value <= bound
                    };
                    if !ok {
                        out.push(format!("{at}: {value} is past its {keyword} {bound}"));
                    }
                }
            }
            "minLength" => {
                let shortest = usize::try_from(rule.as_u64().unwrap()).unwrap();
                if let Some(text) = instance.as_str()
                    && text.chars().count() < shortest
                {
                    out.push(format!("{at}: {text:?} is too short"));
                }
            }
            "minItems" | "maxItems" => {
                if let Some(items) = instance.as_array() {
                    let bound = usize::try_from(rule.as_u64().unwrap()).unwrap();
                    if (keyword == "minItems" && items.len() < bound)
                        || (keyword == "maxItems" && items.len() > bound)
                    {
                        out.push(format!(
                            "{at}: {} items, past its {keyword} {bound}",
                            items.len()
                        ));
                    }
                }
            }
            "required" => {
                if let Some(object) = instance.as_object() {
                    for name in rule.as_array().unwrap().iter().filter_map(Value::as_str) {
                        if !object.contains_key(name) {
                            out.push(format!("{at}: {name} is missing"));
                        }
                    }
                }
            }
            "items" => {
                if let Some(items) = instance.as_array() {
                    for (index, item) in items.iter().enumerate() {
                        problems(root, rule, item, &format!("{at}/{index}"), out);
                    }
                }
            }
            "properties" | "patternProperties" | "additionalProperties" => {
                // All three together, once, under `properties` or whichever
                // of them the schema has first.
                let first = ["properties", "patternProperties", "additionalProperties"]
                    .into_iter()
                    .find(|name| schema.contains_key(*name));
                if first == Some(keyword.as_str()) {
                    fields(root, schema, instance, at, out);
                }
            }
            "anyOf" | "oneOf" => {
                let passing = rule
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|option| {
                        let mut inner = Vec::new();
                        problems(root, option, instance, at, &mut inner);
                        inner.is_empty()
                    })
                    .count();
                let ok = if keyword == "oneOf" {
                    passing == 1
                } else {
                    passing >= 1
                };
                if !ok {
                    out.push(format!("{at}: {passing} of the {keyword} options fit"));
                }
            }
            "$schema" | "title" | "description" | "default" | "$defs" | "uniqueItems" | "enum"
            | "const" => {}
            other => panic!("the test validator does not know `{other}`"),
        }
    }
}

/// An object's fields under `properties`, `patternProperties` and
/// `additionalProperties`.
fn fields(
    root: &Value,
    schema: &serde_json::Map<String, Value>,
    instance: &Value,
    at: &str,
    out: &mut Vec<String>,
) {
    let Some(object) = instance.as_object() else {
        return;
    };
    let named = schema.get("properties").and_then(Value::as_object);
    let patterns: Vec<(regex::Regex, &Value)> = schema
        .get("patternProperties")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(pattern, rule)| (regex::Regex::new(pattern).unwrap(), rule))
        .collect();
    for (name, value) in object {
        let here = format!("{at}/{name}");
        let mut matched = false;
        if let Some(rule) = named.and_then(|named| named.get(name)) {
            matched = true;
            problems(root, rule, value, &here, out);
        }
        for (pattern, rule) in &patterns {
            if pattern.is_match(name) {
                matched = true;
                problems(root, rule, value, &here, out);
            }
        }
        if !matched {
            match schema.get("additionalProperties") {
                Some(Value::Bool(false)) => out.push(format!("{here}: not a field here")),
                Some(rule) => problems(root, rule, value, &here, out),
                None => {}
            }
        }
    }
}

fn is(instance: &Value, kind: &str) -> bool {
    match kind {
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "string" => instance.is_string(),
        "boolean" => instance.is_boolean(),
        "null" => instance.is_null(),
        "number" => instance.is_number(),
        "integer" => instance.is_i64() || instance.is_u64(),
        other => panic!("no JSON type {other}"),
    }
}

/// Equal as JSON Schema means it: 1 and 1.0 are the same number.
fn same(left: &Value, right: &Value) -> bool {
    match (left.as_f64(), right.as_f64()) {
        (Some(left), Some(right)) => left.to_bits() == right.to_bits(),
        _ => left == right,
    }
}

fn check(schema: &Value, instance: &Value) -> Vec<String> {
    let mut out = Vec::new();
    problems(schema, schema, instance, "", &mut out);
    out
}

/// Every `.scene` and `.prefab` a project in the repository holds, leaving
/// out build output and the fixtures that are wrong on purpose.
fn documents(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !matches!(
                name.as_ref(),
                "target" | "build" | "fixtures" | "node_modules"
            ) && !name.starts_with('.')
            {
                documents(&path, found);
            }
        } else if name.ends_with(".scene") || name.ends_with(".prefab") {
            found.push(path);
        }
    }
}

#[test]
fn every_scene_and_prefab_in_the_repository_fits() {
    let scene = describe(Document::Scene).unwrap();
    let prefab = describe(Document::Prefab).unwrap();
    let mut found = Vec::new();
    documents(crate::repository_root(), &mut found);
    assert!(found.len() > 100, "found only {} documents", found.len());
    let mut failures = Vec::new();
    for path in &found {
        let text = std::fs::read_to_string(path).unwrap();
        let instance: Value = serde_json::from_str(&text).unwrap();
        let schema = if path
            .extension()
            .is_some_and(|extension| extension == "prefab")
        {
            &prefab
        } else {
            &scene
        };
        for problem in check(schema, &instance) {
            failures.push(format!("{}: {problem}", path.display()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn components(components: &Value) -> Vec<String> {
    let scene = describe(Document::Scene).unwrap();
    check(
        &scene,
        &json!({ "format_version": sindri_core::SCENE_FORMAT_VERSION, "entities": [{ "id": "a", "components": components.clone() }] }),
    )
}

#[test]
fn what_an_engine_would_quietly_ignore_it_refuses() {
    assert!(components(&json!({ "sindri.sprite": { "texture": "a.png" } })).is_empty());
    assert!(
        components(&json!({
            "sindri.camera": { "projection": "orthographic", "vertical_size": 6.0 }
        }))
        .is_empty()
    );
    assert!(components(&json!({ "sindri.light2d#halo": { "radius": 2.0 } })).is_empty());
    assert!(
        components(&json!({
            "sindri.physics2d.collider": { "shape": { "shape": "circle", "radius": 1.0 } }
        }))
        .is_empty()
    );

    for mistake in [
        json!({ "sindri.sprit": {} }),
        json!({ "sindri.sprite": { "textur": "a.png" } }),
        json!({ "sindri.camera": { "projection": "fisheye" } }),
        json!({ "sindri.light2d#halo": { "radius": "big" } }),
        json!({ "sindri.physics2d.collider": {
            "pieces": [{ "shape": { "shape": "box", "radius": 1.0 } }]
        } }),
        json!({ "sindri.script": { "source": "a.decay" } }),
        json!({ "sindri.audio.source": { "volume": 3.0 } }),
    ] {
        assert!(!components(&mistake).is_empty(), "{mistake} was accepted");
    }
}

#[test]
fn a_prefab_has_one_format_and_a_scene_migrates_older_ones() {
    let prefab = describe(Document::Prefab).unwrap();
    assert!(check(&prefab, &json!({ "format_version": 1 })).is_empty());
    assert!(!check(&prefab, &json!({ "format_version": 2 })).is_empty());
    let scene = describe(Document::Scene).unwrap();
    assert!(check(&scene, &json!({ "format_version": 3 })).is_empty());
    assert!(!check(&scene, &json!({ "format_version": 99 })).is_empty());
    assert!(!check(&scene, &json!({ "entities": [] })).is_empty());
}
