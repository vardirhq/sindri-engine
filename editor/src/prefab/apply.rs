//! Writing what one instance changed into the prefab it is an instance of.

use std::collections::BTreeMap;

use serde_json::Value;
use sindri_core::{
    EntityOverride, PrefabDocument, PrefabLibrary, SceneEntity, SceneEntityId, apply_override,
};

/// The prefab with `overrides` made part of it.
///
/// An override of one of the prefab's own entities is written into that
/// entity. One naming an entity of a prefab nested in this one is written as
/// an override of the nested instance, so the nested prefab itself is left
/// alone: applying a changed coin in a chest changes the chest's coin, not
/// every coin. `prefabs` is where a nested instance's own root is looked up,
/// because that is the key an override of it is written under.
///
/// The instance's own name and place are not the prefab's to take, and are
/// never in `overrides` for its root.
#[must_use]
pub fn applied(
    prefab: &PrefabDocument,
    overrides: &BTreeMap<SceneEntityId, EntityOverride>,
    prefabs: &dyn PrefabLibrary,
) -> PrefabDocument {
    let mut prefab = prefab.clone();
    for (path, changes) in overrides {
        if let Some(entity) = prefab.entities.iter_mut().find(|entity| &entity.id == path) {
            if entity.prefab.is_some() {
                apply_to_nested_root(entity, changes, prefabs);
            } else {
                apply_override(entity, changes);
            }
            continue;
        }
        // Inside a nested instance: `loot/sparkle` is `sparkle` of `loot`.
        let owner = prefab.entities.iter_mut().find_map(|entity| {
            let rest = path
                .as_str()
                .strip_prefix(entity.id.as_str())?
                .strip_prefix('/')?;
            let key = SceneEntityId::new(rest).ok()?;
            Some((entity.prefab.as_mut()?, key))
        });
        if let Some((nested, key)) = owner {
            merge_into(nested.overrides.entry(key).or_default(), changes);
        }
    }
    prefab
}

/// A change to a nested instance's own root: its name, place and switch are
/// the instance entity's, and what it carries is an override of the nested
/// prefab's root.
fn apply_to_nested_root(
    entity: &mut SceneEntity,
    changes: &EntityOverride,
    prefabs: &dyn PrefabLibrary,
) {
    if changes.name.is_some() {
        entity.name.clone_from(&changes.name);
    }
    if changes.transform_3d.is_some() {
        entity.transform_3d = changes.transform_3d;
    }
    if let Some(disabled) = changes.disabled {
        entity.disabled = disabled;
    }
    let Some(nested) = entity.prefab.as_mut() else {
        return;
    };
    if changes.components.is_empty() {
        return;
    }
    let Some(root) = prefabs
        .prefab(&nested.source)
        .and_then(|inner| inner.root().ok())
        .map(|root| root.id.clone())
    else {
        return;
    };
    merge_into(
        nested.overrides.entry(root).or_default(),
        &EntityOverride {
            components: changes.components.clone(),
            ..EntityOverride::default()
        },
    );
}

/// Adds `incoming` to an override that may already say something.
///
/// Later wins, field by field; a component patch is composed with the one
/// already there, so a key the new patch removes stays removed.
fn merge_into(existing: &mut EntityOverride, incoming: &EntityOverride) {
    if incoming.name.is_some() {
        existing.name.clone_from(&incoming.name);
    }
    if incoming.transform_3d.is_some() {
        existing.transform_3d = incoming.transform_3d;
    }
    if incoming.disabled.is_some() {
        existing.disabled = incoming.disabled;
    }
    for (type_name, patch) in &incoming.components {
        compose(
            existing
                .components
                .entry(type_name.clone())
                .or_insert(Value::Null),
            patch,
        );
    }
}

/// `first` then `second`, as one merge patch.
fn compose(first: &mut Value, second: &Value) {
    match (first, second) {
        (Value::Object(first), Value::Object(second)) => {
            for (key, value) in second {
                compose(first.entry(key.clone()).or_insert(Value::Null), value);
            }
        }
        (first, second) => *first = second.clone(),
    }
}
