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
    fn reconcile_instances(&mut self, placed: &[(EntityId, SceneEntity)], label: String) {
        let mut rehearsal = self.world.clone();
        let mut buffer = CommandBuffer::new();
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
        if let Err(error) = self
            .history
            .apply(buffer.into_transaction(label), &mut self.world)
        {
            self.report(error.to_string());
        }
        self.selection.retain_live(&self.world);
    }

    /// Brings every instance up to the prefabs in `changed`, keeping what
    /// each instance overrode.
    ///
    /// The instances are written down against the prefabs they were made
    /// from *before* those change, which is what makes an override survive:
    /// it is the difference from the old prefab, carried onto the new one.
    pub(super) fn update_instances(
        &mut self,
        changed: Vec<(String, PrefabDocument)>,
        cleared: Option<EntityId>,
    ) {
        let Some(mut placed) = self.placed_instances() else {
            return;
        };
        if let Some(cleared) = cleared {
            for (root, entity) in &mut placed {
                if *root == cleared
                    && let Some(instance) = &mut entity.prefab
                {
                    instance.overrides.clear();
                }
            }
        }
        let label = match changed.as_slice() {
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
        self.reconcile_instances(&placed, label);
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

    /// Puts an instance back to exactly what its prefab says.
    ///
    /// `only` narrows it to one entity of the instance, or one component of
    /// that entity; its name, place and parent stay the instance's own.
    pub(super) fn revert_instance(
        &mut self,
        root: EntityId,
        only: Option<(SceneEntityId, Option<String>)>,
    ) {
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
        match &only {
            None => instance.overrides.clear(),
            Some((path, None)) => {
                instance.overrides.remove(path);
            }
            Some((path, Some(type_name))) => {
                if let Some(changes) = instance.overrides.get_mut(path) {
                    changes.components.remove(type_name);
                }
            }
        }
        let label = match only {
            Some((_, Some(type_name))) => format!("Revert {type_name}"),
            _ => format!("Revert {} to its prefab", file_name(&instance.source)),
        };
        self.reconcile_instances(&[(root, placed)], label);
    }

    /// Writes what an instance overrides into its prefab, and brings every
    /// instance of that prefab up to date with it.
    ///
    /// The file is written straight away, and that write is not something
    /// Undo takes back: Undo returns the instances in this scene to what they
    /// were, and the prefab stays as applied.
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
        let updated = prefab::applied(&current, &instance.overrides, self.file.prefabs());
        let Some(path) = self.file.prefabs().path_for(&instance.source) else {
            return;
        };
        let written = updated
            .to_canonical_json()
            .map_err(|error| error.to_string())
            .and_then(|json| std::fs::write(&path, json).map_err(|error| error.to_string()));
        if let Err(error) = written {
            self.report(format!("{} was not written: {error}", path.display()));
            return;
        }
        self.console
            .info(format!("Applied to {}", file_name(&instance.source)));
        self.update_instances(vec![(instance.source, updated)], Some(root));
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

/// What to call a prefab in a label: its file's name.
pub(super) fn file_name(source: &str) -> &str {
    source.rsplit('/').next().unwrap_or(source)
}
