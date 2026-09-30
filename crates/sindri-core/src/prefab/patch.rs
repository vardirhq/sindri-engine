//! JSON merge patches: what an instance's override of a component is.
//!
//! RFC 7386, because it is the smallest thing that says "these fields differ"
//! in the same notation as the component it patches. An object merges key by
//! key, `null` removes a key, and anything else replaces the value outright.
//!
//! RFC 7386 replaces a list outright too, which would make an instance that
//! changed one tile of a prefab's tilemap own the whole map. So a list that
//! changed in few places is patched by index instead, in one extension:
//!
//! ```json
//! { "$items": { "57": 3, "58": { "solid": false } }, "$length": 120 }
//! ```
//!
//! `$items` patches the elements it names — an object merges into an object
//! element, anything else replaces it — and `$length`, when present, cuts the
//! list short or pads it with `null` first. A list that changed in most places
//! is still written whole, because that is shorter and easier to read. Keys
//! beginning with `$` are reserved inside a patch for this reason.

use serde_json::{Map, Value};

/// The key that patches a list's elements by index.
pub const LIST_ITEMS: &str = "$items";
/// The key that sets a list's length.
pub const LIST_LENGTH: &str = "$length";

/// Applies `patch` to `target`.
pub fn apply_merge_patch(target: &mut Value, patch: &Value) {
    let Value::Object(fields) = patch else {
        *target = patch.clone();
        return;
    };
    if is_list_patch(fields) {
        apply_list_patch(target, fields);
        return;
    }
    if !target.is_object() {
        *target = Value::Object(Map::new());
    }
    let Value::Object(target) = target else {
        unreachable!("replaced with an object above");
    };
    for (key, value) in fields {
        if value.is_null() {
            target.remove(key);
        } else {
            apply_merge_patch(target.entry(key.clone()).or_insert(Value::Null), value);
        }
    }
}

fn is_list_patch(fields: &Map<String, Value>) -> bool {
    !fields.is_empty()
        && fields
            .keys()
            .all(|key| key == LIST_ITEMS || key == LIST_LENGTH)
}

fn apply_list_patch(target: &mut Value, fields: &Map<String, Value>) {
    if !target.is_array() {
        *target = Value::Array(Vec::new());
    }
    let Value::Array(items) = target else {
        unreachable!("replaced with a list above");
    };
    if let Some(length) = fields
        .get(LIST_LENGTH)
        .and_then(Value::as_u64)
        .and_then(|length| usize::try_from(length).ok())
    {
        items.resize(length, Value::Null);
    }
    let Some(Value::Object(changes)) = fields.get(LIST_ITEMS) else {
        return;
    };
    for (index, change) in changes {
        let Ok(index) = index.parse::<usize>() else {
            continue;
        };
        if index >= items.len() {
            items.resize(index + 1, Value::Null);
        }
        if change.is_object() && items[index].is_object() {
            apply_merge_patch(&mut items[index], change);
        } else {
            items[index] = change.clone();
        }
    }
}

/// The patch that turns `base` into `modified`, or `None` when they are equal.
///
/// The inverse of [`apply_merge_patch`] for every value it can express. The
/// one it cannot is a `null` *inside* an object in `modified`, which a patch
/// would read as "remove"; component payloads do not store nulls, and one that
/// did would come back without the key, which is how the rest of the engine
/// reads it anyway.
#[must_use]
pub fn merge_patch_between(base: &Value, modified: &Value) -> Option<Value> {
    if base == modified {
        return None;
    }
    match (base, modified) {
        (Value::Object(base), Value::Object(modified)) => Some(object_patch(base, modified)),
        (Value::Array(base), Value::Array(modified)) => {
            Some(list_patch(base, modified).unwrap_or_else(|| Value::Array(modified.clone())))
        }
        _ => Some(modified.clone()),
    }
}

fn object_patch(base: &Map<String, Value>, modified: &Map<String, Value>) -> Value {
    let mut patch = Map::new();
    for (key, before) in base {
        match modified.get(key) {
            None => {
                patch.insert(key.clone(), Value::Null);
            }
            Some(after) => {
                if let Some(change) = merge_patch_between(before, after) {
                    patch.insert(key.clone(), change);
                }
            }
        }
    }
    for (key, after) in modified {
        if !base.contains_key(key) {
            patch.insert(key.clone(), after.clone());
        }
    }
    Value::Object(patch)
}

/// The by-index patch between two lists, or `None` when the whole list is the
/// shorter thing to write.
fn list_patch(base: &[Value], modified: &[Value]) -> Option<Value> {
    let mut items = Map::new();
    for (index, after) in modified.iter().enumerate() {
        let change = match base.get(index) {
            Some(before) if before == after => continue,
            Some(before) if before.is_object() && after.is_object() => {
                merge_patch_between(before, after)?
            }
            _ => after.clone(),
        };
        items.insert(index.to_string(), change);
    }
    if base.is_empty() || items.len() * 2 > modified.len() {
        return None;
    }
    let mut patch = Map::new();
    if !items.is_empty() {
        patch.insert(LIST_ITEMS.to_owned(), Value::Object(items));
    }
    if modified.len() != base.len() {
        patch.insert(LIST_LENGTH.to_owned(), Value::from(modified.len()));
    }
    Some(Value::Object(patch))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn applied(target: Value, patch: &Value) -> Value {
        let mut target = target;
        apply_merge_patch(&mut target, patch);
        target
    }

    #[test]
    fn a_patch_merges_objects_and_replaces_everything_else() {
        let base = json!({ "tint": [1.0, 1.0, 1.0, 1.0], "layer": 3, "size": { "x": 1, "y": 2 } });
        let patch = json!({ "tint": [1.0, 0.0, 0.0, 1.0], "layer": null, "size": { "y": 4 } });
        assert_eq!(
            applied(base, &patch),
            json!({ "tint": [1.0, 0.0, 0.0, 1.0], "size": { "x": 1, "y": 4 } })
        );
    }

    #[test]
    fn the_patch_between_two_values_rebuilds_the_second() {
        let cases = [
            (
                json!({ "a": 1, "b": { "c": 2, "d": 3 } }),
                json!({ "a": 1, "b": { "c": 5 } }),
            ),
            (
                json!({ "a": [1, 2] }),
                json!({ "a": [1, 2, 3], "e": "new" }),
            ),
            (json!(3), json!({ "now": "an object" })),
            (json!({ "gone": true }), json!({})),
        ];
        for (base, modified) in cases {
            let patch = merge_patch_between(&base, &modified).expect("they differ");
            assert_eq!(
                applied(base.clone(), &patch),
                modified,
                "{base} to {modified}"
            );
        }
    }

    #[test]
    fn a_list_changed_in_one_place_is_patched_by_index() {
        let base = json!({ "tiles": [0, 0, 0, 0, 0, 0, 0, 0] });
        let modified = json!({ "tiles": [0, 0, 7, 0, 0, 0, 0, 0] });
        let patch = merge_patch_between(&base, &modified).unwrap();
        assert_eq!(patch, json!({ "tiles": { "$items": { "2": 7 } } }));
        assert_eq!(applied(base, &patch), modified);
    }

    #[test]
    fn a_list_patch_can_shorten_lengthen_and_merge_into_elements() {
        let base = json!([{ "a": 1, "b": 2 }, 2, 3, 4, 5, 6]);
        let cases = [
            json!([{ "a": 1, "b": 9 }, 2, 3, 4, 5]),
            json!([{ "a": 1, "b": 2 }, 2, 3, 4, 5, 6, 7]),
            json!([{ "a": 1 }, 2, 3, 4, 5, 6]),
        ];
        for modified in cases {
            let patch = merge_patch_between(&base, &modified).unwrap();
            assert!(patch.is_object(), "patched by index: {patch}");
            assert_eq!(applied(base.clone(), &patch), modified, "{patch}");
        }
    }

    #[test]
    fn a_list_changed_in_most_places_is_written_whole() {
        let base = json!([1, 2, 3, 4]);
        let modified = json!([9, 8, 7, 4]);
        assert_eq!(merge_patch_between(&base, &modified), Some(modified));
    }

    #[test]
    fn equal_values_need_no_patch() {
        let value = json!({ "tint": [1.0, 1.0, 1.0, 1.0] });
        assert_eq!(merge_patch_between(&value, &value), None);
    }
}
