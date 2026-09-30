//! JSON merge patches: what an instance's override of a component is.
//!
//! RFC 7386, because it is the smallest thing that says "these fields differ"
//! in the same notation as the component it patches. An object merges key by
//! key, `null` removes a key, and anything else replaces the value outright —
//! so a list is overridden whole. That last rule is a real limit: an instance
//! cannot change one tile of a prefab's tilemap without owning the whole list.
//! It is also what keeps an override readable, and a per-index patch format is
//! a second notation nobody would write by hand.

use serde_json::{Map, Value};

/// Applies `patch` to `target`.
pub fn apply_merge_patch(target: &mut Value, patch: &Value) {
    let Value::Object(fields) = patch else {
        *target = patch.clone();
        return;
    };
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

/// The patch that turns `base` into `modified`, or `None` when they are equal.
///
/// The inverse of [`apply_merge_patch`] for every value it can express. The
/// one it cannot is a `null` *inside* `modified`, which a patch would read as
/// "remove"; component payloads do not store nulls, and one that did would
/// come back without the key, which is how the rest of the engine reads it
/// anyway.
#[must_use]
pub fn merge_patch_between(base: &Value, modified: &Value) -> Option<Value> {
    if base == modified {
        return None;
    }
    let (Value::Object(base), Value::Object(modified)) = (base, modified) else {
        return Some(modified.clone());
    };
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
    fn equal_values_need_no_patch() {
        let value = json!({ "tint": [1.0, 1.0, 1.0, 1.0] });
        assert_eq!(merge_patch_between(&value, &value), None);
    }
}
