//! What the editor does with a placed prefab instance: bring it up to date,
//! revert it, apply it to its prefab, or unpack it.
//!
//! Every one of them is the same move made from a different starting point.
//! Each instance is written down as the reference a save would write — its
//! prefab and its overrides — the reference or the prefab is changed, and the
//! instance is reconciled with what it now expands to. The world changes
//! through commands, so each is one undo step and the entities keep their
//! handles.

use sindri_core::{
    CommandBuffer, EntityId, PrefabDocument, PrefabLibrary, SceneEntity, SceneEntityId,
    WorldCommand,
};

use crate::prefab;

use super::prefab_writes::PendingWrite;

use super::EditorApp;

impl EditorApp {
    /// Every entity in the world that is the root of an instance.
    fn instance_roots(&self) -> Vec<EntityId> {
        self.world
            .entities()
            .filter(|(entity, data)| {
                data.prefab.as_ref().is_some_and(|link| link.root)
                    && self.world.instance_root(*entity) == Some(*entity)
            })
            .map(|(entity, _)| entity)
            .collect()
    }

    /// Every instance as its reference, against the prefabs held now.
    fn placed_instances(&mut self) -> Option<Vec<(EntityId, SceneEntity)>> {
        let mut placed = Vec::new();
        for root in self.instance_roots() {
            match self.world.instance_entity(root, self.file.prefabs()) {
                Ok(entity) => placed.push((root, entity)),
                Err(error) => {
                    self.report(error.to_string());
                    return None;
                }
            }
        }
        Some(placed)
    }

    /// Makes each instance what its reference now expands to, as one step.
    ///
    /// `wrote`, when the step also wrote a prefab file, is that write and the
    /// instance it was applied from. The step is recorded even when nothing in
    /// the scene had to change, which is usual after an Apply, so that Undo
    /// has a step to take the file back with.
    fn reconcile_instances(
        &mut self,
        placed: &[(EntityId, SceneEntity)],
        label: String,
        wrote: Option<(EntityId, PendingWrite)>,
    ) {
        let mut rehearsal = self.world.clone();
        let mut buffer = CommandBuffer::new();
        let mut writes = Vec::new();
        if let Some((anchor, write)) = wrote {
            let link = self.world.get(anchor).and_then(|data| data.prefab.clone());
            buffer.push(WorldCommand::SetPrefabLink {
                entity: anchor,
                link,
            });
            writes.push(write);
        }
        for (root, entity) in placed {
            let members = self.world.instance_members(*root);
            if let Err(error) = prefab::reconcile(
                &self.world,
                &mut rehearsal,
                &members,
                entity,
                self.file.prefabs(),
                &mut buffer,
            ) {
                self.report(error);
                return;
            }
        }
        if buffer.is_empty() {
            return;
        }
        self.history.break_merge_run();
        let before = self.history.revision();
        if let Err(error) = self
            .history
            .apply(buffer.into_transaction(label), &mut self.world)
        {
            self.report(error.to_string());
        }
        self.record_prefab_writes(writes, before);
        self.selection.retain_live(&self.world);
    }

    /// Brings every instance up to the prefabs in `changed`, keeping what
    /// each instance overrode.
    ///
    /// The instances are written down against the prefabs they were made
    /// from *before* those change, which is what makes an override survive:
    /// it is the difference from the old prefab, carried onto the new one.
    /// `applied`, after an Apply, is the instance the change came from — whose
    /// overrides are now the prefab's — and the file the Apply wrote.
    pub(super) fn update_instances(
        &mut self,
        changed: Vec<(String, PrefabDocument)>,
        applied: Option<(EntityId, PendingWrite)>,
    ) {
        let Some(mut placed) = self.placed_instances() else {
            return;
        };
        if let Some((cleared, _)) = &applied {
            for (root, entity) in &mut placed {
                if root == cleared
                    && let Some(instance) = &mut entity.prefab
                {
                    instance.overrides.clear();
                    instance.removed.clear();
                }
            }
        }
        let label = match changed.as_slice() {
            [(id, _)] if applied.is_some() => format!("Apply to {}", file_name(id)),
            [(id, _)] => format!("Update instances of {}", file_name(id)),
            _ => "Update prefab instances".to_owned(),
        };
        for (id, document) in changed {
            if let Err(error) = self.file.prefabs_mut().read_placed_by(&document.entities) {
                self.report(error);
                return;
            }
            self.file.prefabs_mut().replace(&id, document);
        }
        self.reconcile_instances(&placed, label, applied);
    }

    /// Follows edits to prefab files made outside this scene, once the file
    /// watcher has read them.
    pub(super) fn follow_prefab_changes(&mut self) {
        if !self.authoring_enabled() {
            return;
        }
        let read = self.scripts.prefabs();
        let changed: Vec<(String, PrefabDocument)> = read
            .ids()
            .filter_map(|id| Some((id.to_owned(), read.get(id)?.clone())))
            .filter(|(id, document)| self.file.prefabs_mut().heard(id, document))
            .collect();
        if !changed.is_empty() {
            self.update_instances(changed, None);
        }
    }

    /// Puts an instance, or part of it, back to what its prefab says.
    ///
    /// Its name, place and parent stay the instance's own whatever the scope.
    pub(super) fn revert_instance(&mut self, root: EntityId, scope: Revert) {
        let mut placed = match self.world.instance_entity(root, self.file.prefabs()) {
            Ok(placed) => placed,
            Err(error) => {
                self.report(error.to_string());
                return;
            }
        };
        let Some(instance) = placed.prefab.as_mut() else {
            return;
        };
        let label = match &scope {
            Revert::Component(_, type_name) => format!("Revert {type_name}"),
            Revert::Removed(path) => format!("Restore {}", path.as_str()),
            Revert::All | Revert::Entity(_) => {
                format!("Revert {} to its prefab", file_name(&instance.source))
            }
        };
        match scope {
            Revert::All => {
                instance.overrides.clear();
                instance.removed.clear();
            }
            Revert::Entity(path) => {
                instance.overrides.remove(&path);
            }
            Revert::Component(path, type_name) => {
                if let Some(changes) = instance.overrides.get_mut(&path) {
                    changes.components.remove(&type_name);
                }
            }
            Revert::Removed(path) => {
                instance.removed.remove(&path);
            }
        }
        self.reconcile_instances(&[(root, placed)], label, None);
    }

    /// Writes what an instance overrides into its prefab, and brings every
    /// instance of that prefab up to date with it.
    ///
    /// The file is written straight away, and Undo takes it back with the
    /// instances, unless the file has been changed since.
    pub(super) fn apply_instance(&mut self, root: EntityId) {
        let placed = match self.world.instance_entity(root, self.file.prefabs()) {
            Ok(placed) => placed,
            Err(error) => {
                self.report(error.to_string());
                return;
            }
        };
        let Some(instance) = placed.prefab else {
            return;
        };
        let Some(current) = self.file.prefabs().prefab(&instance.source).cloned() else {
            return;
        };
        let updated = prefab::applied(&current, &instance, self.file.prefabs());
        let Some(path) = self.file.prefabs().path_for(&instance.source) else {
            return;
        };
        let write = match PendingWrite::write(&path, &instance.source, &updated) {
            Ok(write) => write,
            Err(error) => {
                self.report(error);
                return;
            }
        };
        self.console
            .info(format!("Applied to {}", file_name(&instance.source)));
        self.update_instances(vec![(instance.source, updated)], Some((root, write)));
    }

    /// Makes an instance's entities the scene's own, no longer linked to the
    /// prefab: they save as themselves, and a change to the prefab no longer
    /// reaches them.
    pub(super) fn unpack_instance(&mut self, root: EntityId) {
        let mut buffer = CommandBuffer::new();
        for entity in self.world.instance_members(root) {
            buffer.push(WorldCommand::SetPrefabLink { entity, link: None });
        }
        self.history.break_merge_run();
        if let Err(error) = self.history.apply(
            buffer.into_transaction("Unpack prefab instance"),
            &mut self.world,
        ) {
            self.report(error.to_string());
        }
    }
}

/// How much of an instance to put back to its prefab.
pub(super) enum Revert {
    /// Everything: overrides and removals.
    All,
    /// What one of its entities overrides.
    Entity(SceneEntityId),
    /// One component of one of its entities.
    Component(SceneEntityId, String),
    /// One entity it removed, brought back.
    Removed(SceneEntityId),
}

/// What to call a prefab in a label: its file's name.
pub(super) fn file_name(source: &str) -> &str {
    source.rsplit('/').next().unwrap_or(source)
}
