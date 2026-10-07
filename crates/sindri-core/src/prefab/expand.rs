//! Turning instances into the entities their prefabs describe, and back.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::{PrefabDocument, PrefabError, SceneDocument, SceneEntity, SceneEntityId};

use super::instance::{EntityOverride, PrefabLibrary, PrefabLink};
use super::patch::{apply_merge_patch, merge_patch_between};

/// How deep prefabs may nest inside one another.
///
/// Far past anything authored on purpose, and there to turn a mistake that
/// recursion would take down the process with into an error naming the chain.
pub const MAX_PREFAB_NESTING: usize = 16;

/// One entity of an expanded list, and the instance it came from, if any.
#[derive(Clone, Debug, PartialEq)]
pub struct ExpandedEntity {
    pub entity: SceneEntity,
    pub link: Option<PrefabLink>,
    /// Namespaced original root IDs retained for runtime-local references.
    /// These aliases are expansion metadata and are not serialized.
    pub aliases: BTreeSet<SceneEntityId>,
}

/// Every instance in `entities` replaced by the entities its prefab describes.
///
/// An instance keeps its own ID for the prefab's root; everything under the
/// root is `<instance>/<id in the prefab>`, so two instances of one prefab
/// never collide and an entity a scene parents under an instance's child can
/// name it. A nested instance expands the same way inside its prefab, which is
/// why an override can name `turret/barrel`.
///
/// # Errors
/// A prefab the library does not hold, one that contains itself, or nesting
/// deeper than [`MAX_PREFAB_NESTING`].
pub fn expand_entities(
    entities: &[SceneEntity],
    library: &dyn PrefabLibrary,
) -> Result<Vec<ExpandedEntity>, PrefabError> {
    let expanded = expand_within(entities, library, &mut Vec::new())?;
    validate_aliases(&expanded)?;
    Ok(expanded)
}

fn expand_within(
    entities: &[SceneEntity],
    library: &dyn PrefabLibrary,
    chain: &mut Vec<String>,
) -> Result<Vec<ExpandedEntity>, PrefabError> {
    let mut expanded = Vec::with_capacity(entities.len());
    for entity in entities {
        let Some(instance) = &entity.prefab else {
            expanded.push(ExpandedEntity {
                entity: entity.clone(),
                link: None,
                aliases: BTreeSet::new(),
            });
            continue;
        };
        if chain.contains(&instance.source) {
            let mut cycle = chain.clone();
            cycle.push(instance.source.clone());
            return Err(PrefabError::Cycle(cycle));
        }
        if chain.len() >= MAX_PREFAB_NESTING {
            return Err(PrefabError::TooDeep(MAX_PREFAB_NESTING));
        }
        let prefab = library
            .prefab(&instance.source)
            .ok_or_else(|| PrefabError::Missing(instance.source.clone()))?;
        chain.push(instance.source.clone());
        let inner = expand_within(&prefab.entities, library, chain)?;
        chain.pop();

        let root_key = inner
            .iter()
            .find(|inner| inner.entity.parent.is_none())
            .map(|inner| inner.entity.id.clone())
            .ok_or(PrefabError::NoRoot)?;
        let gone = removed_keys(&inner, &instance.removed, &root_key);
        for part in inner {
            let key = part.entity.id.clone();
            if gone.contains(&key) {
                continue;
            }
            expanded.push(place_part(part, entity, &root_key));
        }
    }
    Ok(expanded)
}

/// Places one expanded member, preserving nested aliases and override semantics.
fn place_part(
    part: ExpandedEntity,
    instance_entity: &SceneEntity,
    root_key: &SceneEntityId,
) -> ExpandedEntity {
    let instance = instance_entity
        .prefab
        .as_ref()
        .expect("an instance is being expanded");
    let key = part.entity.id.clone();
    let placed = |key: &SceneEntityId| {
        if key == root_key {
            instance_entity.id.clone()
        } else {
            instance_path(&instance_entity.id, key)
        }
    };
    let mut aliases: BTreeSet<_> = part
        .aliases
        .iter()
        .map(|alias| instance_path(&instance_entity.id, alias))
        .collect();
    if key == *root_key {
        aliases.insert(instance_path(&instance_entity.id, &key));
    }
    let is_root = key == *root_key;
    let mut placed_entity = part.entity;
    placed_entity.id = placed(&key);
    placed_entity.parent = if is_root {
        instance_entity.parent.clone()
    } else {
        placed_entity.parent.as_ref().map(&placed)
    };
    if let Some(changes) = instance.overrides.get(&key) {
        apply_override(&mut placed_entity, changes);
    }
    if is_root {
        // What the scene says about the instance itself wins over
        // everything the prefab and its overrides said.
        if instance_entity.name.is_some() {
            placed_entity.name.clone_from(&instance_entity.name);
        }
        if instance_entity.transform_3d.is_some() {
            placed_entity.transform_3d = instance_entity.transform_3d;
        }
        placed_entity.disabled |= instance_entity.disabled;
        placed_entity.editor.clone_from(&instance_entity.editor);
    } else {
        // A prefab's editor state is about the prefab as a document,
        // and says nothing about one of its instances; the instance's
        // own is in its override.
        placed_entity.editor = instance
            .overrides
            .get(&key)
            .map(|changes| changes.editor.clone())
            .unwrap_or_default();
    }
    placed_entity.prefab = None;
    ExpandedEntity {
        entity: placed_entity,
        aliases: aliases.clone(),
        link: Some(PrefabLink {
            source: instance.source.clone(),
            path: key,
            root: is_root,
            aliases,
        }),
    }
}

/// The keys an instance does without: the ones it names, and everything under
/// them. The root is never among them.
fn removed_keys(
    inner: &[ExpandedEntity],
    removed: &BTreeSet<SceneEntityId>,
    root_key: &SceneEntityId,
) -> BTreeSet<SceneEntityId> {
    let mut gone: BTreeSet<SceneEntityId> = removed
        .iter()
        .filter(|key| *key != root_key)
        .cloned()
        .collect();
    if gone.is_empty() {
        return gone;
    }
    // Parents come before children in an expanded list only by accident, so
    // this runs until nothing more is added.
    loop {
        let before = gone.len();
        for part in inner {
            if part
                .entity
                .parent
                .as_ref()
                .is_some_and(|parent| gone.contains(parent))
            {
                gone.insert(part.entity.id.clone());
            }
        }
        if gone.len() == before {
            return gone;
        }
    }
}

/// The ID an entity of an instance has in the document holding the instance.
///
/// # Panics
/// Never: both parts are non-empty IDs, so the joined one is too.
#[must_use]
pub fn instance_path(instance: &SceneEntityId, key: &SceneEntityId) -> SceneEntityId {
    SceneEntityId::new(format!("{}/{}", instance.as_str(), key.as_str()))
        .expect("joining two non-empty IDs is non-empty")
}

impl PrefabDocument {
    /// This prefab with every instance inside it expanded.
    ///
    /// What a spawn creates and what an instance of this prefab is compared
    /// against. The entities are the prefab's own, at the paths an override
    /// names them by.
    ///
    /// # Errors
    /// As [`expand_entities`].
    pub fn expanded(&self, library: &dyn PrefabLibrary) -> Result<Self, PrefabError> {
        if self.entities.iter().all(|entity| entity.prefab.is_none()) {
            return Ok(self.clone());
        }
        let entities = expand_entities(&self.entities, library)?
            .into_iter()
            .map(|expanded| expanded.entity)
            .collect();
        Ok(Self {
            format_version: self.format_version,
            metadata: self.metadata.clone(),
            entities,
        })
    }

    /// Whether any entity in this prefab is itself an instance.
    #[must_use]
    pub fn has_instances(&self) -> bool {
        self.entities.iter().any(|entity| entity.prefab.is_some())
    }
}

impl SceneDocument {
    /// This scene with every instance made from its prefab, as plain entities.
    ///
    /// For a host that plays a scene and never saves it: what it loads is
    /// what the instances expand to, and every loading path it already has
    /// works unchanged. The editor keeps the links instead, through
    /// [`crate::World::from_scene_with`], because it writes the scene back.
    ///
    /// # Errors
    /// As [`expand_entities`], and a scene whose entities are invalid once
    /// expanded — one parented under an instance's child it does not have.
    pub fn expanded(&self, library: &dyn PrefabLibrary) -> Result<Self, PrefabError> {
        if self.entities.iter().all(|entity| entity.prefab.is_none()) {
            return Ok(self.clone());
        }
        let entities: Vec<SceneEntity> = expand_entities(&self.entities, library)?
            .into_iter()
            .map(|expanded| expanded.entity)
            .collect();
        crate::scene::validate_entities(&entities)?;
        let mut document = Self {
            format_version: self.format_version,
            metadata: self.metadata.clone(),
            entities,
        };
        document.canonicalize();
        Ok(document)
    }
}

/// Applies what an instance changed to one entity of its prefab.
pub fn apply_override(entity: &mut SceneEntity, changes: &EntityOverride) {
    if changes.name.is_some() {
        entity.name.clone_from(&changes.name);
    }
    if changes.transform_3d.is_some() {
        entity.transform_3d = changes.transform_3d;
    }
    if let Some(disabled) = changes.disabled {
        entity.disabled = disabled;
    }
    for (type_name, patch) in &changes.components {
        if patch.is_null() {
            entity.components.remove(type_name);
        } else {
            apply_merge_patch(
                entity
                    .components
                    .entry(type_name.clone())
                    .or_insert(Value::Null),
                patch,
            );
        }
    }
}

/// What turns `base` into `current`, as an instance would write it.
///
/// A name, transform or switch is written whole when it differs, and each
/// component as the merge patch between the two. An instance that changed
/// nothing gives an empty override, which is not written at all: an override
/// is a difference, and setting a field to what the prefab already says is
/// not one.
#[must_use]
pub fn override_between(base: &SceneEntity, current: &SceneEntity) -> EntityOverride {
    let mut components = BTreeMap::new();
    for (type_name, before) in &base.components {
        match current.components.get(type_name) {
            None => {
                components.insert(type_name.clone(), Value::Null);
            }
            Some(after) => {
                if let Some(patch) = merge_patch_between(before, after) {
                    components.insert(type_name.clone(), patch);
                }
            }
        }
    }
    for (type_name, after) in &current.components {
        if !base.components.contains_key(type_name) {
            components.insert(type_name.clone(), after.clone());
        }
    }
    EntityOverride {
        name: (current.name != base.name)
            .then(|| current.name.clone())
            .flatten(),
        transform_3d: (current.transform_3d != base.transform_3d)
            .then_some(current.transform_3d)
            .flatten(),
        disabled: (current.disabled != base.disabled).then_some(current.disabled),
        components,
        editor: BTreeMap::new(),
    }
}

/// Canonical paths shadow aliases; competing aliases must not depend on order.
fn validate_aliases(expanded: &[ExpandedEntity]) -> Result<(), PrefabError> {
    let canonical: BTreeSet<_> = expanded.iter().map(|part| &part.entity.id).collect();
    let mut aliases = BTreeSet::new();
    for part in expanded {
        for alias in &part.aliases {
            if !canonical.contains(alias) && !aliases.insert(alias) {
                return Err(PrefabError::AmbiguousRootAlias(alias.clone()));
            }
        }
    }
    Ok(())
}
