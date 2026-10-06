//! Creating entities in a world from an authored prefab.

use std::collections::HashMap;

use crate::prefab::{NoPrefabs, PrefabLibrary};
use crate::{EntityId, PrefabDocument, PrefabError, SceneEntityId};

use super::{EntityData, World, WorldError};

/// An authored path scoped to a single runtime spawn, never a saved scene ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrefabIdentity {
    /// Generation-checked root handle distinguishing this spawn from every other.
    pub root: EntityId,
    /// Path in the expanded prefab, including its original root identity.
    pub path: SceneEntityId,
}

/// What one spawn produced.
#[derive(Clone, Debug)]
pub struct SpawnedPrefab {
    /// The prefab's single root, and what a caller holds on to.
    pub root: EntityId,
    /// Every entity the spawn created, including the root, in document order.
    ///
    /// A caller undoing a spawn needs all of them, and a caller reaching into
    /// a named child needs to be able to find it without walking the world.
    pub entities: Vec<EntityId>,
    /// Which runtime entity each authored identity became.
    ///
    /// The prefab's identities are the *prefab's*, not the world's: spawning
    /// the same prefab twice makes two entities that shared an authored name
    /// and share nothing else.
    pub by_source_id: HashMap<SceneEntityId, EntityId>,
}

impl World {
    /// Creates the prefab's entities and returns what they became.
    ///
    /// Spawned entities carry **no** `source_id`. A prefab's identities name
    /// entities inside the prefab and are not stable identities in this world:
    /// two instances would collide on every one of them, and a scene saved
    /// with the collision would refuse to load. `assign_missing_source_ids`
    /// remains how a runtime entity earns a stable identity, which is a
    /// decision about persisting a world rather than about spawning.
    ///
    /// The prefab is validated first, so a document with several roots is
    /// refused rather than half-spawned. Nothing reaches the world until the
    /// whole document has been checked.
    ///
    /// # Errors
    /// An invalid prefab, or one with a prefab nested in it, which needs the
    /// library [`World::spawn_prefab_from`] is given.
    pub fn spawn_prefab(&mut self, prefab: &PrefabDocument) -> Result<SpawnedPrefab, WorldError> {
        self.spawn_prefab_from(prefab, &NoPrefabs)
    }

    /// [`World::spawn_prefab`], making every prefab nested in it from
    /// `prefabs`.
    ///
    /// A nested instance's entities are keyed in [`SpawnedPrefab::
    /// by_source_id`] by their path in the prefab, `turret/barrel`.
    ///
    /// # Errors
    /// An invalid prefab, or a nested one `prefabs` does not hold.
    pub fn spawn_prefab_from(
        &mut self,
        prefab: &PrefabDocument,
        prefabs: &dyn PrefabLibrary,
    ) -> Result<SpawnedPrefab, WorldError> {
        prefab.validate()?;
        let expanded = prefab.expanded(prefabs)?;
        let prefab = &expanded;
        let root_id = prefab.root()?.id.clone();

        let mut by_source_id = HashMap::with_capacity(prefab.entities.len());
        let mut entities = Vec::with_capacity(prefab.entities.len());
        for entity in &prefab.entities {
            let runtime = self.spawn(EntityData {
                source_id: None,
                name: entity.name.clone(),
                transform_3d: entity.transform_3d,
                components: entity.components.clone(),
                disabled: entity.disabled,
                // Editor-only state describes the prefab in the editor, not the
                // instance in a running world. Carrying it would put a
                // selection highlight and a fold state on every bullet.
                editor: std::collections::BTreeMap::new(),
                ..EntityData::default()
            });
            by_source_id.insert(entity.id.clone(), runtime);
            entities.push(runtime);
        }

        for entity in &prefab.entities {
            if let Some(parent) = &entity.parent {
                self.set_parent(by_source_id[&entity.id], Some(by_source_id[parent]))?;
            }
        }

        let root = by_source_id[&root_id];
        for (path, &entity) in &by_source_id {
            self.get_mut(entity)
                .expect("entity created by this spawn")
                .prefab_identity = Some(PrefabIdentity {
                root,
                path: path.clone(),
            });
        }

        Ok(SpawnedPrefab {
            root,
            entities,
            by_source_id,
        })
    }

    /// Resolves a local authored path inside the owner's spawned prefab.
    /// Searches enclosing path namespaces first, then the prefab's own root
    /// namespace. Missing targets never escape to a different instance or scene.
    /// Activity is left to the caller; a disabled local target still owns its name.
    #[must_use]
    pub fn prefab_entity(&self, owner: EntityId, target: &str) -> Option<EntityId> {
        let identity = self.get(owner)?.prefab_identity.as_ref()?;
        if target.is_empty() || !self.contains(identity.root) {
            return None;
        }
        let find = |path: &str| {
            self.entities().find_map(|(entity, data)| {
                data.prefab_identity.as_ref().and_then(|candidate| {
                    (candidate.root == identity.root && candidate.path.as_str() == path)
                        .then_some(entity)
                })
            })
        };
        let mut namespace = identity.path.as_str();
        while let Some((prefix, _)) = namespace.rsplit_once('/') {
            if let Some(entity) = find(&format!("{prefix}/{target}")) {
                return Some(entity);
            }
            namespace = prefix;
        }
        find(target)
    }
}

impl From<PrefabError> for WorldError {
    fn from(error: PrefabError) -> Self {
        Self::InvalidPrefab(error)
    }
}
