//! Writing a world back as a scene, with each prefab instance written as the
//! reference it was loaded from rather than as copies of its entities.

use std::collections::{BTreeMap, HashMap};

use crate::prefab::{
    NoPrefabs, PrefabError, PrefabInstance, PrefabLibrary, instance_path, override_between,
};
use crate::{EntityId, SceneDocument, SceneEntity, SceneEntityId};

use super::{World, WorldError};

impl World {
    /// Serializes this world back into a canonical scene document.
    ///
    /// Stable IDs are preserved rather than regenerated, so saving a loaded
    /// scene reproduces the authored identities. Entities spawned at runtime
    /// have no stable ID and are reported instead of being silently dropped or
    /// given an arbitrary one; call [`World::assign_missing_source_ids`] first
    /// to give them persistent identities.
    ///
    /// # Errors
    /// An entity with no stable ID, or a prefab instance: writing one back
    /// needs its prefab, which [`World::to_scene_with`] is given.
    pub fn to_scene(&self) -> Result<SceneDocument, WorldError> {
        self.to_scene_with(&NoPrefabs)
    }

    /// Serializes this world, writing each prefab instance as a reference to
    /// its prefab and the overrides it made.
    ///
    /// The overrides are worked out here, as the difference between what each
    /// entity of the instance is now and what its prefab says it is. Nothing
    /// tracks them as they are made, so an edit that puts a value back to the
    /// prefab's stops being an override without anyone having to say so.
    ///
    /// # Errors
    /// As [`World::to_scene`]; a prefab `prefabs` does not hold; and an
    /// instance whose entities no longer match its prefab's — one deleted or
    /// moved to another parent — which an override cannot say.
    pub fn to_scene_with(&self, prefabs: &dyn PrefabLibrary) -> Result<SceneDocument, WorldError> {
        let mut entities = Vec::with_capacity(self.len);
        let mut instances: BTreeMap<EntityId, Vec<(SceneEntityId, SceneEntity)>> = BTreeMap::new();
        for (entity_id, data) in self.entities() {
            let written = self.written_entity(entity_id)?;
            match (self.instance_root(entity_id), &data.prefab) {
                (Some(root), Some(link)) => instances
                    .entry(root)
                    .or_default()
                    .push((link.path.clone(), written)),
                _ => entities.push(written),
            }
        }
        for (root, members) in instances {
            entities.push(self.collapse_instance(root, members, prefabs)?);
        }

        let mut document = SceneDocument {
            format_version: crate::SCENE_FORMAT_VERSION,
            metadata: self.metadata.clone(),
            entities,
        };
        document.canonicalize();
        document.validate()?;
        Ok(document)
    }

    /// One instance, as the scene entity a save would write it as.
    ///
    /// What the editor compares an instance's prefab against, reverts to, and
    /// carries across when the prefab changes underneath it.
    ///
    /// # Errors
    /// As [`World::to_scene_with`], for this instance alone.
    pub fn instance_entity(
        &self,
        root: EntityId,
        prefabs: &dyn PrefabLibrary,
    ) -> Result<SceneEntity, WorldError> {
        let members = self
            .instance_members(root)
            .into_iter()
            .map(|member| {
                let path = self
                    .get(member)
                    .and_then(|data| data.prefab.as_ref())
                    .map(|link| link.path.clone())
                    .ok_or(WorldError::InvalidEntity(member))?;
                Ok((path, self.written_entity(member)?))
            })
            .collect::<Result<Vec<_>, WorldError>>()?;
        self.collapse_instance(root, members, prefabs)
    }

    /// One entity as a scene writes it, before instances are collapsed.
    fn written_entity(&self, entity: EntityId) -> Result<SceneEntity, WorldError> {
        let data = self.get(entity).ok_or(WorldError::InvalidEntity(entity))?;
        let source_id = data
            .source_id
            .clone()
            .ok_or(WorldError::UnstableEntity(entity))?;
        let parent = match data.parent {
            Some(parent) => Some(
                self.get(parent)
                    .and_then(|parent_data| parent_data.source_id.clone())
                    .ok_or(WorldError::UnstableEntity(parent))?,
            ),
            None => None,
        };
        Ok(SceneEntity {
            name: data.name.clone(),
            parent,
            transform_3d: data.transform_3d,
            components: data.components.clone(),
            disabled: data.disabled,
            editor: data.editor.clone(),
            ..SceneEntity::new(source_id)
        })
    }

    /// The root of the instance `entity` belongs to, itself included.
    ///
    /// The nearest ancestor that is the root of an instance of the same
    /// prefab. An entity carrying a link with no such ancestor — one moved out
    /// of its instance — belongs to none, and is written as itself.
    #[must_use]
    pub fn instance_root(&self, entity: EntityId) -> Option<EntityId> {
        let source = &self.get(entity)?.prefab.as_ref()?.source;
        let mut cursor = Some(entity);
        while let Some(current) = cursor {
            let data = self.get(current)?;
            if data
                .prefab
                .as_ref()
                .is_some_and(|link| link.root && &link.source == source)
            {
                return Some(current);
            }
            cursor = data.parent;
        }
        None
    }

    /// Every entity of the instance rooted at `root`, root first.
    #[must_use]
    pub fn instance_members(&self, root: EntityId) -> Vec<EntityId> {
        let mut members = vec![root];
        let mut index = 0;
        while let Some(&current) = members.get(index) {
            index += 1;
            let Some(data) = self.get(current) else {
                continue;
            };
            members.extend(
                data.children
                    .iter()
                    .copied()
                    .filter(|child| self.instance_root(*child) == Some(root)),
            );
        }
        members
    }

    /// One instance, as the scene entity that stands for it.
    fn collapse_instance(
        &self,
        root: EntityId,
        members: Vec<(SceneEntityId, SceneEntity)>,
        prefabs: &dyn PrefabLibrary,
    ) -> Result<SceneEntity, WorldError> {
        let source = self
            .get(root)
            .and_then(|data| data.prefab.as_ref())
            .map(|link| link.source.clone())
            .ok_or(WorldError::InvalidEntity(root))?;
        let prefab = prefabs
            .prefab(&source)
            .ok_or_else(|| PrefabError::Missing(source.clone()))?
            .expanded(prefabs)?;
        let root_key = prefab.root()?.id.clone();
        let mut by_path: HashMap<SceneEntityId, SceneEntity> = members.into_iter().collect();
        let current_root = by_path
            .get(&root_key)
            .cloned()
            .ok_or(WorldError::InvalidEntity(root))?;
        let reshaped = |detail: String| WorldError::InstanceReshaped {
            instance: current_root.id.clone(),
            detail,
        };

        let mut overrides = BTreeMap::new();
        let mut written = SceneEntity {
            parent: current_root.parent.clone(),
            editor: current_root.editor.clone(),
            ..SceneEntity::new(current_root.id.clone())
        };
        for base in &prefab.entities {
            let current = by_path
                .remove(&base.id)
                .ok_or_else(|| reshaped(format!("'{}' was removed", base.id.as_str())))?;
            let mut changes = override_between(base, &current);
            if base.id == root_key {
                // The instance's own name, place and switch are written on the
                // instance, which is where somebody reading the scene looks.
                if current.name != base.name {
                    written.name = current.name;
                }
                // Where the instance stands, unless it stands where the prefab
                // says, which is the one place that is not a difference.
                if current.transform_3d != base.transform_3d {
                    written.transform_3d = current.transform_3d;
                }
                written.disabled = current.disabled && !base.disabled;
                changes.name = None;
                changes.transform_3d = None;
                if !base.disabled || current.disabled {
                    changes.disabled = None;
                }
            } else {
                let expected = base.parent.as_ref().map(|parent| {
                    if parent == &root_key {
                        written.id.clone()
                    } else {
                        instance_path(&written.id, parent)
                    }
                });
                if current.parent != expected {
                    return Err(reshaped(format!(
                        "'{}' was moved to another parent",
                        base.id.as_str()
                    )));
                }
            }
            if !changes.is_empty() {
                overrides.insert(base.id.clone(), changes);
            }
        }
        if let Some(extra) = by_path.keys().next() {
            return Err(reshaped(format!(
                "'{}' is no longer in the prefab",
                extra.as_str()
            )));
        }
        written.prefab = Some(PrefabInstance { source, overrides });
        Ok(written)
    }
}
