//! Owned reference choices so drawing a panel never borrows or edits its world.
use std::collections::BTreeMap;

use serde_json::Value;
use sindri_core::{EntityId, World};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReferenceStatus {
    Unbound,
    Missing,
    Active(String),
    Inactive(String),
}

#[derive(Clone, Debug)]
pub struct EntityChoice {
    pub reference: String,
    pub label: String,
}

#[derive(Clone, Debug, Default)]
pub struct EntityReferences {
    pub choices: Vec<EntityChoice>,
    statuses: BTreeMap<String, ReferenceStatus>,
}

impl EntityReferences {
    /// Captures choices and diagnostics using the runtime's resolution rules.
    #[must_use]
    pub fn new(world: &World, owner: EntityId, payloads: &BTreeMap<String, Value>) -> Self {
        let mut references = Self::default();
        let status = |target: &str| {
            world.resolve_entity_reference(owner, target).map_or(
                ReferenceStatus::Missing,
                |entity| {
                    let label = world
                        .get(entity)
                        .and_then(|data| data.name.clone())
                        .unwrap_or_else(|| target.to_owned());
                    if world.is_active(entity) {
                        ReferenceStatus::Active(label)
                    } else {
                        ReferenceStatus::Inactive(label)
                    }
                },
            )
        };
        for (entity, data) in world.entities() {
            if let Some(target) = world.entity_reference(owner, entity) {
                let label = data
                    .name
                    .as_ref()
                    .map_or_else(|| target.clone(), |name| format!("{name} ({target})"));
                references.choices.push(EntityChoice {
                    reference: target.clone(),
                    label,
                });
                references.statuses.insert(target.clone(), status(&target));
            }
        }
        references
            .choices
            .sort_by(|a, b| a.reference.cmp(&b.reference));
        for payload in payloads.values() {
            collect_strings(payload, &mut |target| {
                references
                    .statuses
                    .insert(target.to_owned(), status(target));
            });
        }
        references
    }

    #[must_use]
    pub fn status(&self, reference: &str) -> ReferenceStatus {
        if reference.is_empty() {
            ReferenceStatus::Unbound
        } else {
            self.statuses
                .get(reference)
                .cloned()
                .unwrap_or(ReferenceStatus::Missing)
        }
    }
}

fn collect_strings(value: &Value, visit: &mut impl FnMut(&str)) {
    match value {
        Value::String(text) => visit(text),
        Value::Array(items) => {
            for item in items {
                collect_strings(item, visit);
            }
        }
        Value::Object(fields) => {
            for value in fields.values() {
                collect_strings(value, visit);
            }
        }
        _ => {}
    }
}
