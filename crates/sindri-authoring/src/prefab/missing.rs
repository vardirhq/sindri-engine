//! Instances whose prefab cannot be read, held as stand-ins until it can.
//!
//! A scene naming a prefab that has been deleted, renamed or broken used to
//! refuse to open, which left the one tool that could fix it unable to. It now
//! opens: each such instance becomes a placeholder entity where it stood,
//! carrying the instance exactly as the scene wrote it, and is written back
//! exactly so when the scene is saved. Entities the scene hung under one of
//! the instance's children hang under the placeholder meanwhile, and go back
//! under the child they named.

use std::collections::BTreeSet;

use serde_json::{Value, json};
use sindri_core::{PrefabInstance, SceneDocument, SceneEntityId};

/// Where a placeholder keeps the instance it stands in for.
pub const MISSING_PREFAB: &str = "sindri.editor.missing_prefab";
/// Where an entity under a placeholder keeps the parent it really names.
pub const MISSING_PARENT: &str = "sindri.editor.missing_parent";

/// `document` with every instance of a prefab in `missing` a placeholder.
#[must_use]
pub fn stand_in(document: &SceneDocument, missing: &BTreeSet<String>) -> SceneDocument {
    let mut document = document.clone();
    let mut standing: Vec<SceneEntityId> = Vec::new();
    for entity in &mut document.entities {
        let Some(instance) = entity
            .prefab
            .take_if(|instance| missing.contains(&instance.source))
        else {
            continue;
        };
        let file = instance
            .source
            .rsplit('/')
            .next()
            .unwrap_or(&instance.source);
        entity.editor.insert(
            MISSING_PREFAB.to_owned(),
            json!({ "instance": instance, "name": entity.name }),
        );
        entity.name = Some(format!(
            "{} ({file} is missing)",
            entity.name.as_deref().unwrap_or(entity.id.as_str())
        ));
        standing.push(entity.id.clone());
    }
    for entity in &mut document.entities {
        let Some(parent) = entity.parent.clone() else {
            continue;
        };
        let owner = standing.iter().find(|placeholder| {
            parent
                .as_str()
                .strip_prefix(placeholder.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
        });
        if let Some(owner) = owner {
            entity
                .editor
                .insert(MISSING_PARENT.to_owned(), Value::from(parent.as_str()));
            entity.parent = Some(owner.clone());
        }
    }
    document
}

/// `document` as the scene should be written: every placeholder the instance
/// it stands in for, and every entity under one back under the child it named.
pub fn restore(document: &mut SceneDocument) {
    for entity in &mut document.entities {
        if let Some(held) = entity.editor.remove(MISSING_PREFAB)
            && let Ok(instance) = serde_json::from_value::<PrefabInstance>(held["instance"].clone())
        {
            entity.name = held["name"].as_str().map(str::to_owned);
            // A placeholder has nothing of its own to keep: what the instance
            // is made of is in the prefab, which is what is missing.
            entity.components.clear();
            entity.prefab = Some(instance);
        }
        if let Some(parent) = entity.editor.remove(MISSING_PARENT)
            && let Some(parent) = parent.as_str().and_then(|p| SceneEntityId::new(p).ok())
        {
            entity.parent = Some(parent);
        }
    }
}
