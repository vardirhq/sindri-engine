//! Writing what one instance changed into the prefab it is an instance of.

use std::collections::BTreeMap;

use serde_json::Value;
use sindri_core::{
    EntityOverride, PrefabDocument, PrefabInstance, PrefabLibrary, SceneEntity, SceneEntityId,
    apply_merge_patch, apply_override,
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
/// An entity the instance removed is removed from the prefab, with everything
/// under it — or, inside a nested instance, recorded as that instance's
/// removal. The instance's own name and place are not the prefab's to take,
/// and are never in its overrides for its root; neither is its editor state.
#[must_use]
pub fn applied(
    prefab: &PrefabDocument,
    instance: &PrefabInstance,
    prefabs: &dyn PrefabLibrary,
) -> PrefabDocument {
    let mut prefab = prefab.clone();
    let overrides: BTreeMap<&SceneEntityId, EntityOverride> = instance
        .overrides
        .iter()
        .map(|(path, changes)| {
            let changes = EntityOverride {
                editor: BTreeMap::new(),
                ..changes.clone()
            };
            (path, changes)
        })
        .filter(|(_, changes)| !changes.is_empty())
        .collect();
    for (path, changes) in overrides {
        let changes = &changes;
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
    for path in &instance.removed {
        remove_from(&mut prefab, path);
    }
    prefab
}

/// Takes an entity, and what is under it, out of the prefab — or records it
/// as removed from the nested instance it belongs to.
fn remove_from(prefab: &mut PrefabDocument, path: &SceneEntityId) {
    if prefab.entities.iter().any(|entity| &entity.id == path) {
        let mut gone = vec![path.clone()];
        let mut index = 0;
        while let Some(parent) = gone.get(index).cloned() {
            index += 1;
            gone.extend(
                prefab
                    .entities
                    .iter()
                    .filter(|entity| entity.parent.as_ref() == Some(&parent))
                    .map(|entity| entity.id.clone()),
            );
        }
        prefab.entities.retain(|entity| !gone.contains(&entity.id));
        return;
    }
    let owner = prefab.entities.iter_mut().find_map(|entity| {
        let rest = path
            .as_str()
            .strip_prefix(entity.id.as_str())?
            .strip_prefix('/')?;
        Some((entity.prefab.as_mut()?, SceneEntityId::new(rest).ok()?))
    });
    if let Some((nested, key)) = owner {
        nested.removed.insert(key);
    }
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
///
/// Two patches of an object compose key by key. A patch applied to a value
/// the first one set outright — a whole list, say, patched by index — is
/// applied to that value, since the value is what the first patch means.
fn compose(first: &mut Value, second: &Value) {
    match (&mut *first, second) {
        // Two patches of one list: the later one's elements and length win,
        // each element composed with the earlier patch of it.
        (Value::Object(earlier), Value::Object(later))
            if is_list_patch(earlier) && is_list_patch(later) =>
        {
            for (key, value) in later {
                if key == sindri_core::LIST_ITEMS
                    && let (Some(Value::Object(items)), Value::Object(changes)) =
                        (earlier.get_mut(key), value)
                {
                    for (index, change) in changes {
                        compose(items.entry(index.clone()).or_insert(Value::Null), change);
                    }
                } else {
                    earlier.insert(key.clone(), value.clone());
                }
            }
        }
        (Value::Object(earlier), Value::Object(later)) if !is_list_patch(earlier) => {
            for (key, value) in later {
                compose(earlier.entry(key.clone()).or_insert(Value::Null), value);
            }
        }
        (Value::Null | Value::Object(_), _) => *first = second.clone(),
        (_, _) => apply_merge_patch(first, second),
    }
}

fn is_list_patch(patch: &serde_json::Map<String, Value>) -> bool {
    patch
        .keys()
        .any(|key| key == sindri_core::LIST_ITEMS || key == sindri_core::LIST_LENGTH)
}
