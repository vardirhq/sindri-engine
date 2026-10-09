//! Edits made while a scene plays, and what Stop does with them.
//!
//! A run plays a copy of the scene, and Stop puts the scene back as it was
//! when Play was pressed. An edit made while playing therefore changes the
//! running world at once — that is the point of making it there: tune a jump
//! and try it — and is recorded against the run rather than the scene's
//! history. At Stop each recorded edit is offered back, and the ones kept are
//! applied to the restored scene as ordinary, undoable history entries.
//!
//! What comes back is what the person changed, not what the run left behind.
//! A component edit is kept as the fields it changed, laid over the scene's
//! own value: a script that wrote another field of the same component while
//! the game ran does not smuggle that value into the scene. An edit whose
//! target the restored scene does not have — something the run spawned, or
//! something made while playing whose making was not kept — is explained,
//! never dropped in silence.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};
use sindri_core::{
    CommandBuffer, CommandError, CommandHistory, EntityId, Transaction, Transform3D, World,
    WorldCommand,
};

/// One edit made during a run, as it was asked for.
#[derive(Clone, Debug)]
pub struct RunEdit {
    label: String,
    commands: Vec<WorldCommand>,
    /// Each component the edit wrote, as it was before the edit's first
    /// write: what "the fields it changed" are measured from.
    before: BTreeMap<(EntityId, String), Option<Value>>,
    /// Each transform the edit wrote, as it was before the edit's first
    /// write, for the same reason.
    transforms: BTreeMap<EntityId, Option<Transform3D>>,
    /// What each entity it wrote to was called, for saying which one.
    names: BTreeMap<EntityId, String>,
    merge_key: Option<String>,
    /// How many steps of the run had run when it was made.
    step: u64,
}

impl RunEdit {
    /// What the edit was, as the history would label it.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// The edits a run has collected so far.
#[derive(Debug)]
pub struct RunEdits {
    edits: Vec<RunEdit>,
    /// Applies them to the running world; records nothing, because undoing
    /// into a run is not something Stop could then offer back.
    applier: CommandHistory,
    /// Whether the next edit with the same merge key continues the last one:
    /// a drag is one edit, not one per frame.
    merging: bool,
}

impl Default for RunEdits {
    fn default() -> Self {
        Self {
            edits: Vec::new(),
            applier: CommandHistory::with_limit(0),
            merging: false,
        }
    }
}

impl RunEdits {
    /// Applies `transaction` to the running `world` and records it.
    ///
    /// # Errors
    /// The command layer's refusal, with the world unchanged.
    pub fn apply(
        &mut self,
        transaction: Transaction,
        world: &mut World,
    ) -> Result<(), CommandError> {
        self.apply_at(transaction, world, 0)
    }

    /// The same, made when `step` steps of the run had run.
    ///
    /// # Errors
    /// The command layer's refusal, with the world unchanged.
    pub fn apply_at(
        &mut self,
        transaction: Transaction,
        world: &mut World,
        step: u64,
    ) -> Result<(), CommandError> {
        let mut before = BTreeMap::new();
        let mut transforms = BTreeMap::new();
        let mut names = BTreeMap::new();
        for command in transaction.commands() {
            if let Some(entity) = command.entity()
                && let Some(data) = world.get(entity)
            {
                names
                    .entry(entity)
                    .or_insert_with(|| data.name.clone().unwrap_or_else(|| "an entity".to_owned()));
            }
            if let WorldCommand::SetTransform3D { entity, .. } = command {
                transforms
                    .entry(*entity)
                    .or_insert_with(|| world.get(*entity).and_then(|data| data.transform_3d));
            }
            if let WorldCommand::SetComponent {
                entity, type_name, ..
            }
            | WorldCommand::RemoveComponent {
                entity, type_name, ..
            } = command
            {
                before
                    .entry((*entity, type_name.clone()))
                    .or_insert_with(|| {
                        world
                            .get(*entity)
                            .and_then(|data| data.components.get(type_name).cloned())
                    });
            }
        }
        let label = transaction.label().to_owned();
        let commands = transaction.commands().to_vec();
        let merge_key = transaction.merge_key().map(str::to_owned);
        self.applier.apply(transaction, world)?;
        match self.edits.last_mut() {
            Some(last) if self.merging && merge_key.is_some() && last.merge_key == merge_key => {
                last.commands.extend(commands);
                for (key, value) in before {
                    last.before.entry(key).or_insert(value);
                }
                for (entity, transform) in transforms {
                    last.transforms.entry(entity).or_insert(transform);
                }
                for (entity, name) in names {
                    last.names.entry(entity).or_insert(name);
                }
            }
            _ => self.edits.push(RunEdit {
                label,
                commands,
                before,
                transforms,
                names,
                merge_key,
                step,
            }),
        }
        self.merging = true;
        Ok(())
    }

    /// Lets go of the edits made after `step`: the run was taken back there
    /// and carried on, so they are no longer in it.
    pub fn forget_after(&mut self, step: u64) {
        self.edits.retain(|edit| edit.step <= step);
        self.merging = false;
    }

    /// Ends a continuous interaction, so the next edit is its own.
    pub fn break_merge_run(&mut self) {
        self.merging = false;
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.edits.len()
    }

    /// The edits, oldest first, leaving none.
    pub fn take(&mut self) -> Vec<RunEdit> {
        self.merging = false;
        std::mem::take(&mut self.edits)
    }
}

/// What Stop can do with one edit.
#[derive(Clone, Debug)]
pub enum Verdict {
    /// The edit as it applies to the restored scene.
    Keep(Transaction),
    /// Why it cannot be kept, worded for a person.
    Explained(String),
}

/// What each of `edits` would be on the `restored` scene, in order.
///
/// Entities an earlier edit in the list spawned count as there, on the
/// assumption that edit is kept; if it is not, keeping a later one fails
/// and says so then.
///
/// An entity made during the run was given the handle the running world had
/// free, which may be a slot the run had emptied and the restored scene still
/// fills; such a one is moved to a handle the restored scene has free, and
/// every later edit naming it follows.
#[must_use]
pub fn review(edits: &[RunEdit], restored: &World) -> Vec<Verdict> {
    let moved = free_handles(edits, restored);
    let mut made: BTreeSet<EntityId> = BTreeSet::new();
    edits
        .iter()
        .map(|edit| retargeted(edit, &moved))
        .collect::<Vec<_>>()
        .iter()
        .map(|edit| {
            let verdict = review_one(edit, restored, &made);
            for command in &edit.commands {
                if let WorldCommand::Spawn { entity, .. } = command {
                    made.insert(*entity);
                }
            }
            verdict
        })
        .collect()
}

fn review_one(edit: &RunEdit, restored: &World, made: &BTreeSet<EntityId>) -> Verdict {
    let mut spawned_here: BTreeSet<EntityId> = BTreeSet::new();
    let mut commands = CommandBuffer::new();
    for command in &edit.commands {
        if let WorldCommand::Spawn { entity, .. } = command {
            spawned_here.insert(*entity);
            commands.push(command.clone());
            continue;
        }
        let Some(entity) = command.entity() else {
            commands.push(command.clone());
            continue;
        };
        let name = edit.names.get(&entity).map_or("an entity", String::as_str);
        let there = restored.get(entity).is_some() || made.contains(&entity);
        if !there && !spawned_here.contains(&entity) {
            return Verdict::Explained(format!(
                "{name} was made while the scene played, so the scene has no {name} to change"
            ));
        }
        match command {
            WorldCommand::SetComponent {
                entity,
                type_name,
                payload,
            } => {
                let before = edit
                    .before
                    .get(&(*entity, type_name.clone()))
                    .cloned()
                    .flatten();
                let current = restored
                    .get(*entity)
                    .and_then(|data| data.components.get(type_name));
                let payload = match (before, current) {
                    // Added by the edit: the whole value is what was asked for.
                    (None, _) => payload.clone(),
                    (Some(_), None) if spawned_here.contains(entity) || made.contains(entity) => {
                        payload.clone()
                    }
                    (Some(_), None) => {
                        return Verdict::Explained(format!(
                            "{name} had {type_name} only while the scene played"
                        ));
                    }
                    (Some(before), Some(current)) => {
                        let mut patched = current.clone();
                        for (path, value) in changes(&before, payload) {
                            set_at(&mut patched, &path, value);
                        }
                        patched
                    }
                };
                commands.push(WorldCommand::SetComponent {
                    entity: *entity,
                    type_name: type_name.clone(),
                    payload,
                });
            }
            WorldCommand::SetTransform3D {
                entity,
                transform: Some(after),
            } => {
                let before = edit.transforms.get(entity).copied().flatten();
                let current = restored.get(*entity).and_then(|data| data.transform_3d);
                let transform = match (before, current) {
                    (Some(before), Some(current)) => changed_parts(before, *after, current),
                    _ => *after,
                };
                commands.push(WorldCommand::SetTransform3D {
                    entity: *entity,
                    transform: Some(transform),
                });
            }
            WorldCommand::RemoveComponent { entity, type_name } => {
                let has = restored
                    .get(*entity)
                    .is_some_and(|data| data.components.contains_key(type_name));
                if has || spawned_here.contains(entity) || made.contains(entity) {
                    commands.push(command.clone());
                }
            }
            other => {
                commands.push(other.clone());
            }
        }
    }
    Verdict::Keep(commands.into_transaction(edit.label.clone()))
}

/// Where each entity the edits make can be made in `restored`: its own handle
/// when that slot is free there, else the next one that is.
fn free_handles(edits: &[RunEdit], restored: &World) -> BTreeMap<EntityId, EntityId> {
    let mut rehearsal = restored.clone();
    let mut moved = BTreeMap::new();
    for command in edits.iter().flat_map(|edit| &edit.commands) {
        let WorldCommand::Spawn { entity, data } = command else {
            continue;
        };
        if rehearsal.spawn_at(*entity, (**data).clone()).is_err() {
            let free = rehearsal.next_handle();
            if rehearsal.spawn_at(free, (**data).clone()).is_ok() {
                moved.insert(*entity, free);
            }
        }
    }
    moved
}

/// `edit` with every handle in `moved` replaced by where it moved to.
fn retargeted(edit: &RunEdit, moved: &BTreeMap<EntityId, EntityId>) -> RunEdit {
    if moved.is_empty() {
        return edit.clone();
    }
    let to = |entity: EntityId| moved.get(&entity).copied().unwrap_or(entity);
    let mut edit = edit.clone();
    for command in &mut edit.commands {
        match command {
            WorldCommand::Spawn { entity, data } => {
                *entity = to(*entity);
                data.parent = data.parent.map(to);
                for child in &mut data.children {
                    *child = to(*child);
                }
            }
            WorldCommand::SetParent { entity, parent } => {
                *entity = to(*entity);
                *parent = parent.map(to);
            }
            WorldCommand::Restore { root, entities, .. } => {
                *root = to(*root);
                for (entity, data) in entities {
                    *entity = to(*entity);
                    data.parent = data.parent.map(to);
                }
            }
            WorldCommand::SetName { entity, .. }
            | WorldCommand::SetSourceId { entity, .. }
            | WorldCommand::SetTransform3D { entity, .. }
            | WorldCommand::SetComponent { entity, .. }
            | WorldCommand::RemoveComponent { entity, .. }
            | WorldCommand::SetDisabled { entity, .. }
            | WorldCommand::SetEditorEntry { entity, .. }
            | WorldCommand::SetPrefabLink { entity, .. }
            | WorldCommand::Despawn { entity } => *entity = to(*entity),
            WorldCommand::SetSceneName { .. } => {}
        }
    }
    edit.before = std::mem::take(&mut edit.before)
        .into_iter()
        .map(|((entity, kind), value)| ((to(entity), kind), value))
        .collect();
    edit.transforms = std::mem::take(&mut edit.transforms)
        .into_iter()
        .map(|(entity, value)| (to(entity), value))
        .collect();
    edit.names = std::mem::take(&mut edit.names)
        .into_iter()
        .map(|(entity, name)| (to(entity), name))
        .collect();
    edit
}

/// `current` with the parts of a transform that went from `before` to
/// `after` — position, rotation, scale, the layer lock — and no others: an
/// edit to the scale of something the run had moved keeps the scale, not
/// where the run had moved it to.
fn changed_parts(before: Transform3D, after: Transform3D, current: Transform3D) -> Transform3D {
    let mut kept = current;
    if differs(&after.position, &before.position) {
        kept.position = after.position;
    }
    if differs(&after.rotation, &before.rotation) {
        kept.rotation = after.rotation;
    }
    if differs(&after.scale, &before.scale) {
        kept.scale = after.scale;
    }
    if after.z_locked != before.z_locked {
        kept.z_locked = after.z_locked;
    }
    kept
}

/// Whether an edit wrote anything different: exactly, because any change at
/// all is a change the person made.
fn differs(after: &[f32], before: &[f32]) -> bool {
    after
        .iter()
        .zip(before)
        .any(|(after, before)| after.to_bits() != before.to_bits())
}

/// Every place `after` differs from `before`, as a path of object keys and
/// the value now there, `None` where a key was taken away. Arrays and
/// values are compared whole.
fn changes(before: &Value, after: &Value) -> Vec<(Vec<String>, Option<Value>)> {
    let mut found = Vec::new();
    walk(before, after, &mut Vec::new(), &mut found);
    found
}

fn walk(
    before: &Value,
    after: &Value,
    path: &mut Vec<String>,
    found: &mut Vec<(Vec<String>, Option<Value>)>,
) {
    match (before, after) {
        (Value::Object(was), Value::Object(now)) => {
            for (key, value) in now {
                path.push(key.clone());
                match was.get(key) {
                    Some(old) => walk(old, value, path, found),
                    None => found.push((path.clone(), Some(value.clone()))),
                }
                path.pop();
            }
            for key in was.keys().filter(|key| !now.contains_key(*key)) {
                path.push(key.clone());
                found.push((path.clone(), None));
                path.pop();
            }
        }
        _ if before == after => {}
        _ => found.push((path.clone(), Some(after.clone()))),
    }
}

/// Writes `value` at `path` inside `target`, making objects on the way, or
/// takes the key away for `None`.
fn set_at(target: &mut Value, path: &[String], value: Option<Value>) {
    let Some((last, parents)) = path.split_last() else {
        if let Some(value) = value {
            *target = value;
        }
        return;
    };
    let mut at = target;
    for key in parents {
        if !at.is_object() {
            *at = Value::Object(Map::new());
        }
        at = at
            .as_object_mut()
            .expect("made an object above")
            .entry(key.clone())
            .or_insert_with(|| Value::Object(Map::new()));
    }
    if !at.is_object() {
        *at = Value::Object(Map::new());
    }
    let object = at.as_object_mut().expect("made an object above");
    match value {
        Some(value) => {
            object.insert(last.clone(), value);
        }
        None => {
            object.remove(last);
        }
    }
}

#[cfg(test)]
mod tests;
