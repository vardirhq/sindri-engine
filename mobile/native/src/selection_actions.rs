//! Selection and the undoable verbs shared with desktop authoring.

use sindri_core::{CommandBuffer, EntityId, World, WorldCommand};

use crate::Editing;

impl Editing {
    /// Every selected entity, with the primary first.
    pub fn selected_entities(&self) -> Vec<EntityId> {
        self.selected
            .into_iter()
            .chain(self.additional.iter().copied())
            .collect()
    }

    /// List the live scene, including prefab children without persistent scene IDs.
    pub fn scene_objects(&self, world: &World) -> Vec<serde_json::Value> {
        let selected = self.selected_entities();
        let mut objects: Vec<_> = world.entities().map(|(entity, data)| {
            let name = data.name.clone()
                .or_else(|| data.source_id.as_ref().map(|id| id.as_str().to_owned()))
                .unwrap_or_else(|| format!("{entity:?}"));
            serde_json::json!({
                "id": entity.to_bits().to_string(),
                "name": name,
                "selected": selected.contains(&entity),
                "disabled": data.disabled,
                "screen": data.components.keys().any(|name| name.starts_with("sindri.ui.")),
            })
        }).collect();
        objects.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()).then_with(|| a["id"].as_str().cmp(&b["id"].as_str())));
        objects
    }

    /// Select an object using a generation-checked handle from the current scene.
    ///
    /// # Errors
    /// Returns an error if the handle is invalid or no longer refers to a live object.
    pub fn select_object(&mut self, world: &World, id: &str) -> Result<(), String> {
        let bits = id.parse::<u64>().map_err(|_| "Invalid object handle")?;
        let entity = EntityId::from_bits(bits);
        if world.get(entity).is_none() {
            return Err("Object no longer exists".to_owned());
        }
        self.clear_selection();
        self.selected = Some(entity);
        Ok(())
    }

    /// Toggle whether canvas taps add or remove objects from the selection.
    pub fn select_more(&mut self) {
        self.selecting_more = !self.selecting_more;
    }

    /// Add or remove an entity from the selection.
    pub fn toggle_selected(&mut self, entity: EntityId) {
        if self.selected == Some(entity) {
            self.selected = if self.additional.is_empty() {
                None
            } else {
                Some(self.additional.remove(0))
            };
        } else if let Some(index) = self.additional.iter().position(|item| *item == entity) {
            self.additional.remove(index);
        } else if self.selected.is_none() {
            self.selected = Some(entity);
        } else {
            self.additional.push(entity);
        }
        if self.selected.is_none() {
            self.selecting_more = false;
        }
    }

    /// Clear both the selection and the add-to-selection mode.
    pub fn clear_selection(&mut self) {
        self.selected = None;
        self.additional.clear();
        self.selecting_more = false;
    }

    /// Delete selected subtrees in one undoable transaction.
    pub fn delete_selection(&mut self, world: &mut World) -> Result<(), String> {
        let roots = topmost(world, &self.selected_entities());
        if roots.is_empty() {
            return Err("Nothing is selected".to_owned());
        }
        let mut commands = CommandBuffer::new();
        for entity in roots {
            commands.push(WorldCommand::Despawn { entity });
        }
        self.history.break_merge_run();
        self.history
            .apply(commands.into_transaction("Delete selection"), world)
            .map_err(|e| e.to_string())?;
        self.clear_selection();
        Ok(())
    }

    /// Duplicate selected subtrees using the desktop editor's copy semantics.
    pub fn duplicate_selection(&mut self, world: &mut World) -> Result<(), String> {
        let roots = topmost(world, &self.selected_entities());
        if roots.is_empty() {
            return Err("Nothing is selected".to_owned());
        }
        let mut commands = CommandBuffer::new();
        let mut rehearsal = world.clone();
        let mut copies = Vec::new();
        for entity in roots {
            copies.extend(sindri_authoring::duplicate::duplicate_into(
                &mut rehearsal,
                world,
                entity,
                &mut commands,
            ));
        }
        self.history.break_merge_run();
        self.history
            .apply(commands.into_transaction("Duplicate selection"), world)
            .map_err(|e| e.to_string())?;
        self.selected = copies.first().copied();
        self.additional = copies.into_iter().skip(1).collect();
        self.selecting_more = false;
        Ok(())
    }

    /// Forget handles removed by undo or redo.
    pub(crate) fn prune_selection(&mut self, world: &World) {
        self.additional.retain(|id| world.get(*id).is_some());
        if self.selected.is_some_and(|id| world.get(id).is_none()) {
            self.selected = if self.additional.is_empty() {
                None
            } else {
                Some(self.additional.remove(0))
            };
        }
        if self.selected.is_none() {
            self.selecting_more = false;
        }
    }
}

fn topmost(world: &World, selected: &[EntityId]) -> Vec<EntityId> {
    selected
        .iter()
        .copied()
        .filter(|entity| {
            if world.get(*entity).is_none() {
                return false;
            }
            let mut parent = world.get(*entity).and_then(|data| data.parent);
            while let Some(id) = parent {
                if selected.contains(&id) {
                    return false;
                }
                parent = world.get(id).and_then(|data| data.parent);
            }
            true
        })
        .collect()
}
