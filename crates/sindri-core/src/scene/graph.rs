//! What a list of authored entities has to be true of, whatever document holds
//! it.
//!
//! A scene and a prefab disagree about how many roots they may have and about
//! nothing else: identities are unique and non-empty, a parent named is a
//! parent present, transforms are finite, and following parents always reaches
//! a root. Those rules are here rather than in either document, because the
//! second copy of a validator is the one that stops matching the first.

use std::collections::{HashMap, HashSet};

use crate::Transform3D;

use super::document::{SceneEntity, SceneEntityId};
use super::error::SceneError;

/// Checks everything a list of authored entities must satisfy.
///
/// Not the document's format version: that belongs to the document, which is
/// the thing that carries one.
pub(crate) fn validate_entities(entities: &[SceneEntity]) -> Result<(), SceneError> {
    if entities
        .iter()
        .any(|entity| entity.id.as_str().trim().is_empty())
    {
        return Err(SceneError::EmptyEntityId);
    }

    let ids: HashSet<_> = entities.iter().map(|entity| &entity.id).collect();
    if ids.len() != entities.len() {
        return Err(SceneError::DuplicateEntityId);
    }

    for entity in entities {
        if let Some(transform) = &entity.transform_3d
            && !transform_3d_is_finite(transform)
        {
            return Err(SceneError::NonFiniteTransform(entity.id.clone()));
        }
        if entity.prefab.is_some() && !entity.components.is_empty() {
            return Err(SceneError::InstanceComponents(entity.id.clone()));
        }

        if let Some(parent) = &entity.parent {
            if parent == &entity.id {
                return Err(SceneError::HierarchyCycle(entity.id.clone()));
            }
            if !ids.contains(parent) && owning_instance(entities, parent).is_none() {
                return Err(SceneError::MissingParent {
                    entity: entity.id.clone(),
                    parent: parent.clone(),
                });
            }
        }
    }

    reject_hierarchy_cycles(entities, &ids)
}

/// The instance a path like `coin-3/sparkle` names an entity inside, when it
/// is not an entity of the list itself.
///
/// Such an entity exists only once the instance is expanded, so until then the
/// instance answers for it: an entity may be authored under a prefab's child.
fn owning_instance<'a>(
    entities: &'a [SceneEntity],
    path: &SceneEntityId,
) -> Option<&'a SceneEntityId> {
    entities
        .iter()
        .filter(|entity| entity.prefab.is_some())
        .map(|entity| &entity.id)
        .find(|instance| {
            path.as_str()
                .strip_prefix(instance.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
        })
}

/// The entities that name no parent.
pub(crate) fn roots(entities: &[SceneEntity]) -> impl Iterator<Item = &SceneEntity> {
    entities.iter().filter(|entity| entity.parent.is_none())
}

/// Rejects a list where following parents does not always reach a root.
///
/// Each entity used to walk its own ancestors, and each step of that walk
/// searched the whole entity list for the parent it named, so validating a
/// scene cost time proportional to its size squared: ten thousand entities
/// spent about 1.4 seconds here, and every load, save, and canonical
/// serialization pays it.
///
/// Parents resolve through a map instead, and an entity proven to reach a
/// root is remembered, so a chain shared by many entities is walked once
/// rather than once per descendant.
fn reject_hierarchy_cycles(
    entities: &[SceneEntity],
    ids: &HashSet<&SceneEntityId>,
) -> Result<(), SceneError> {
    // A parent inside an instance is under the instance, as far as a cycle
    // is concerned: its own ancestors are the instance's.
    let parents: HashMap<&SceneEntityId, Option<&SceneEntityId>> = entities
        .iter()
        .map(|entity| {
            let parent = entity.parent.as_ref().map(|parent| {
                if ids.contains(parent) {
                    parent
                } else {
                    owning_instance(entities, parent).unwrap_or(parent)
                }
            });
            (&entity.id, parent)
        })
        .collect();

    let mut grounded: HashSet<&SceneEntityId> = HashSet::with_capacity(entities.len());
    let mut walked: HashSet<&SceneEntityId> = HashSet::new();
    let mut path: Vec<&SceneEntityId> = Vec::new();

    for entity in entities {
        walked.clear();
        path.clear();
        let mut cursor = Some(&entity.id);
        while let Some(current) = cursor {
            if grounded.contains(current) {
                break;
            }
            if !walked.insert(current) {
                return Err(SceneError::HierarchyCycle(entity.id.clone()));
            }
            path.push(current);
            cursor = parents.get(current).copied().flatten();
        }
        grounded.extend(path.iter().copied());
    }
    Ok(())
}

fn transform_3d_is_finite(transform: &Transform3D) -> bool {
    transform.position.iter().all(|value| value.is_finite())
        && transform.rotation.iter().all(|value| value.is_finite())
        && transform.scale.iter().all(|value| value.is_finite())
}
