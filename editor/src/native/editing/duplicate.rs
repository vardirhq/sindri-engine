//! Copying an entity and everything under it, as one undoable step.
//!
//! Its own file because the interesting part is not the copying but the
//! handles. `WorldCommand::Spawn` names the handle it spawns at, so a
//! transaction that copies a subtree has to know, before it runs, every handle
//! the world is about to hand out — and `World::next_handle` is a peek rather
//! than an allocation, so asking it four times answers the same thing four
//! times.
//!
//! So the copy is rehearsed. A clone of the world receives exactly the spawns
//! the real one is about to, in the same order, and hands out the handles it
//! will hand out. Cloning a world is what pressing Play already costs, and a
//! duplicate is rarer than a frame.

use std::collections::HashMap;

use sindri_core::{
    CommandBuffer, EntityData, EntityId, PrefabLink, SceneEntityId, Transform3D, World,
    WorldCommand, instance_path,
};

/// Adds the commands that copy `entity` and its descendants beside it, and
/// answers with the handle the copy's root will land on.
///
/// `None` when there is nothing at that handle, which is the only failure this
/// can have: everything else is a spawn the world is about to accept.
///
/// `rehearsal` is a clone of the world that receives exactly the spawns the
/// real one is about to, so it hands out the handles the real one will. It is
/// taken rather than made here because copying a *selection* is one
/// transaction: five copies rehearsed separately would each be told the same
/// next handle and each pick the same unused stable ID, and the transaction
/// would spawn five things on top of each other.
pub(crate) fn duplicate_into(
    rehearsal: &mut World,
    world: &World,
    entity: EntityId,
    buffer: &mut CommandBuffer,
) -> Option<EntityId> {
    let parent = world.get(entity)?.parent;
    let mut copied_roots = HashMap::new();
    Some(copy_into(
        rehearsal,
        world,
        entity,
        parent,
        None,
        buffer,
        &mut copied_roots,
    ))
}

/// Adds the commands that copy `entity`, from `world`, and its descendants
/// under `parent` in the world `rehearsal` stands for, at `placed` when given
/// rather than where the original was. What a paste is: the entity copied
/// from one world, which may be a clipboard's, into another.
pub(crate) fn copy_under(
    rehearsal: &mut World,
    world: &World,
    entity: EntityId,
    parent: Option<EntityId>,
    placed: Option<Transform3D>,
    buffer: &mut CommandBuffer,
) -> Option<EntityId> {
    world.get(entity)?;
    Some(copy_into(
        rehearsal,
        world,
        entity,
        parent,
        placed,
        buffer,
        &mut HashMap::new(),
    ))
}

/// Copies one entity and then everything under it, depth first.
///
/// Parents first, so a child's copy can name the handle its parent's copy was
/// given rather than the one the original had.
///
/// A prefab instance copied whole is a new instance of the same prefab: its
/// root gets a new ID and every entity under it the path under that ID the
/// instance is saved by. Part of an instance copied without its root is no
/// longer part of any instance, and is copied as plain entities.
/// `copied_roots` is the ID each instance root copied so far was given.
fn copy_into(
    rehearsal: &mut World,
    world: &World,
    entity: EntityId,
    parent: Option<EntityId>,
    placed: Option<Transform3D>,
    buffer: &mut CommandBuffer,
    copied_roots: &mut HashMap<EntityId, SceneEntityId>,
) -> EntityId {
    let source = world.get(entity).expect("the caller checked this handle");
    let within = source
        .prefab
        .as_ref()
        .filter(|link| !link.root)
        .and_then(|link| {
            let root_id = copied_roots.get(&world.instance_root(entity)?)?;
            Some(instance_path(root_id, &link.path))
        });
    let is_member = within.is_some();
    let source_id = within.unwrap_or_else(|| unused_id(rehearsal, source.source_id.as_ref()));
    let mut link = source.prefab.clone().filter(|link| link.root || is_member);
    if link.as_ref().is_some_and(|link| link.root) {
        copied_roots.insert(entity, source_id.clone());
    }
    if let Some(link) = &mut link
        && let Some(original_root) = world.instance_root(entity)
        && let Some(original_id) = world
            .get(original_root)
            .and_then(|data| data.source_id.as_ref())
        && let Some(copied_id) = copied_roots.get(&original_root)
    {
        rebase_aliases(link, original_id, copied_id);
    }
    let data = EntityData {
        source_id: Some(source_id),
        // A member keeps its name, which is its prefab's; renaming it would
        // make every copy of an instance override the name it was given.
        name: source.name.as_ref().map(|name| {
            if is_member {
                name.clone()
            } else {
                format!("{name} copy")
            }
        }),
        parent,
        // The copy's own children are spawned by the recursion below; taking
        // the original's list would name entities that are not under it.
        children: Vec::new(),
        transform_3d: placed.or(source.transform_3d),
        components: source.components.clone(),
        // A copy of a switched-off entity is switched off: the copy is of what
        // is there, and one that arrived running would be a surprise in a
        // subtree someone deliberately quietened.
        disabled: source.disabled,
        editor: source.editor.clone(),
        prefab: link,
        // An editor copy must not share the original runtime spawn's scope.
        prefab_identity: None,
        scene_namespace: source.scene_namespace.clone(),
    };
    let handle = rehearsal.spawn(data.clone());
    // The rehearsal spawns, so the real command has a handle to name. Its own
    // child list is rebuilt by `relink_child` when the command runs.
    rehearsal
        .set_parent(handle, parent)
        .expect("a fresh entity accepts the parent its original had");
    buffer.push(WorldCommand::Spawn {
        entity: handle,
        data: Box::new(data),
    });
    for child in &source.children {
        copy_into(
            rehearsal,
            world,
            *child,
            Some(handle),
            None,
            buffer,
            copied_roots,
        );
    }
    handle
}

/// A stable ID like the original's that nothing is using yet.
///
/// Derived from the original rather than generated fresh, because `player-copy`
/// says what it is and `game-object-7` does not — and a stable ID is what a
/// `sindri.grid.occupant` names and what sibling order is sorted by.
fn unused_id(world: &World, original: Option<&SceneEntityId>) -> SceneEntityId {
    let stem = original.map_or_else(|| "game-object".to_owned(), |id| id.as_str().to_owned());
    let mut candidate = format!("{stem}-copy");
    let mut suffix = 2_u32;
    while taken(world, &candidate) {
        candidate = format!("{stem}-copy-{suffix}");
        suffix += 1;
    }
    SceneEntityId::new(candidate).expect("a derived ID is never empty")
}

fn taken(world: &World, candidate: &str) -> bool {
    world.entities().any(|(_, data)| {
        data.source_id
            .as_ref()
            .is_some_and(|id| id.as_str() == candidate)
    })
}

/// Root aliases belong to the copied instance's namespace, never its source.
fn rebase_aliases(link: &mut PrefabLink, original: &SceneEntityId, copied: &SceneEntityId) {
    let prefix = format!("{}/", original.as_str());
    link.aliases = link
        .aliases
        .iter()
        .filter_map(|alias| {
            let local = SceneEntityId::new(alias.as_str().strip_prefix(&prefix)?).ok()?;
            Some(instance_path(copied, &local))
        })
        .collect();
}

#[cfg(test)]
mod reference_tests;
