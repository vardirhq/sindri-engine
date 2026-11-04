//! Bringing a placed instance in line with its prefab, as undoable commands.
//!
//! The runtime makes an instance once, when a scene loads. The editor keeps
//! one alive while its prefab is edited, reverted to, or applied to, and each
//! of those is the same question: here is what the instance should expand to
//! now, so what has to change in the world to get there? The answer is a list
//! of commands rather than a rebuilt world, so the entities keep their handles
//! and the selection and every other undo step keep naming what they named.

use std::collections::{BTreeMap, HashMap};

use sindri_core::{
    CommandBuffer, EntityData, EntityId, ExpandedEntity, PrefabLibrary, SceneEntity, SceneEntityId,
    World, WorldCommand, expand_entities, instance_path,
};

/// Adds the commands that make an instance what `placed` expands to against
/// `prefabs`.
///
/// `members` are the entities the instance has now, from
/// [`World::instance_members`]; none, for an instance being made.
///
/// `rehearsal` is a clone of the world that receives every spawn, so each new
/// entity is given the handle the real world will give it.
///
/// # Errors
/// A prefab `prefabs` does not hold, named.
pub fn reconcile(
    world: &World,
    rehearsal: &mut World,
    members: &[EntityId],
    placed: &SceneEntity,
    prefabs: &dyn PrefabLibrary,
    buffer: &mut CommandBuffer,
) -> Result<(), String> {
    let target = expand_entities(std::slice::from_ref(placed), prefabs)
        .map_err(|error| error.to_string())?;
    let mut handles: HashMap<SceneEntityId, EntityId> = members
        .iter()
        .copied()
        .filter_map(|member| {
            let id = world.get(member)?.source_id.clone()?;
            Some((id, member))
        })
        .collect();
    let wanted: BTreeMap<&SceneEntityId, &ExpandedEntity> = target
        .iter()
        .map(|expanded| (&expanded.entity.id, expanded))
        .collect();

    // Gone first, and only the topmost of what goes: a despawn takes its
    // subtree with it, and despawning a child of something already gone would
    // name a handle that no longer exists.
    for (id, &member) in &handles {
        if wanted.contains_key(id) {
            continue;
        }
        let parent_goes = world
            .get(member)
            .and_then(|data| data.parent)
            .and_then(|parent| world.get(parent))
            .and_then(|parent| parent.source_id.as_ref())
            .is_some_and(|parent| handles.contains_key(parent) && !wanted.contains_key(parent));
        if !parent_goes {
            buffer.push(WorldCommand::Despawn { entity: member });
        }
    }
    handles.retain(|id, _| wanted.contains_key(id));

    for expanded in parents_first(&target) {
        let entity = &expanded.entity;
        let parent = entity.parent.as_ref().and_then(|parent| {
            handles
                .get(parent)
                .copied()
                .or_else(|| world.entity_for_source_id(parent))
        });
        if let Some(&existing) = handles.get(&entity.id) {
            update(world, existing, expanded, parent, buffer);
        } else {
            let data = EntityData {
                source_id: Some(entity.id.clone()),
                name: entity.name.clone(),
                parent,
                transform_3d: entity.transform_3d,
                components: entity.components.clone(),
                disabled: entity.disabled,
                editor: entity.editor.clone(),
                prefab: expanded.link.clone(),
                ..EntityData::default()
            };
            let handle = rehearsal.spawn(data.clone());
            rehearsal
                .set_parent(handle, parent)
                .expect("a fresh entity accepts the parent it was given");
            buffer.push(WorldCommand::Spawn {
                entity: handle,
                data: Box::new(data),
            });
            handles.insert(entity.id.clone(), handle);
        }
    }
    Ok(())
}

/// Puts a new instance of `source` into the world, and answers with its root.
///
/// The instance's ID is the prefab root's own where nothing holds it, and a
/// numbered one after it where something does — `coin`, then `coin-2` —
/// checked against every ID the instance would add, so its children cannot
/// collide either.
///
/// # Errors
/// A prefab `prefabs` does not hold, or one that will not expand.
pub fn spawn_instance(
    rehearsal: &mut World,
    source: &str,
    prefabs: &dyn PrefabLibrary,
    parent: Option<EntityId>,
    transform: Option<sindri_core::Transform3D>,
    buffer: &mut CommandBuffer,
) -> Result<EntityId, String> {
    let prefab = prefabs
        .prefab(source)
        .ok_or_else(|| format!("the prefab {source} is not loaded"))?
        .expanded(prefabs)
        .map_err(|error| error.to_string())?;
    let root = prefab.root().map_err(|error| error.to_string())?;
    let parent_id = parent.and_then(|parent| rehearsal.get(parent)?.source_id.clone());
    let id = unused_instance_id(rehearsal, &root.id, &prefab.entities);
    let placed = SceneEntity {
        parent: parent_id,
        transform_3d: transform.or(root.transform_3d),
        ..SceneEntity::instance(id.clone(), source)
    };
    let world = rehearsal.clone();
    reconcile(&world, rehearsal, &[], &placed, prefabs, buffer)?;
    rehearsal
        .entity_for_source_id(&id)
        .ok_or_else(|| "the instance was not made".to_owned())
}

/// A root ID nothing holds, and under which nothing the instance adds is held.
fn unused_instance_id(
    world: &World,
    authored: &SceneEntityId,
    entities: &[SceneEntity],
) -> SceneEntityId {
    let free = |candidate: &SceneEntityId| {
        world.entity_for_source_id(candidate).is_none()
            && entities
                .iter()
                .filter(|entity| entity.parent.is_some())
                .all(|entity| {
                    world
                        .entity_for_source_id(&instance_path(candidate, &entity.id))
                        .is_none()
                })
    };
    if free(authored) {
        return authored.clone();
    }
    let mut suffix = 2_u32;
    loop {
        let candidate = SceneEntityId::new(format!("{}-{suffix}", authored.as_str()))
            .expect("a derived ID is never empty");
        if free(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

/// Commands for whatever differs between an existing entity and its target.
fn update(
    world: &World,
    entity: EntityId,
    expanded: &ExpandedEntity,
    parent: Option<EntityId>,
    buffer: &mut CommandBuffer,
) {
    let Some(data) = world.get(entity) else {
        return;
    };
    let target = &expanded.entity;
    if data.name != target.name {
        buffer.push(WorldCommand::SetName {
            entity,
            name: target.name.clone(),
        });
    }
    if data.parent != parent {
        buffer.push(WorldCommand::SetParent { entity, parent });
    }
    if data.transform_3d != target.transform_3d {
        buffer.push(WorldCommand::SetTransform3D {
            entity,
            transform: target.transform_3d,
        });
    }
    if data.disabled != target.disabled {
        buffer.push(WorldCommand::SetDisabled {
            entity,
            disabled: target.disabled,
        });
    }
    for (type_name, payload) in &target.components {
        if data.components.get(type_name) != Some(payload) {
            buffer.push(WorldCommand::SetComponent {
                entity,
                type_name: type_name.clone(),
                payload: payload.clone(),
            });
        }
    }
    for type_name in data.components.keys() {
        if !target.components.contains_key(type_name) {
            buffer.push(WorldCommand::RemoveComponent {
                entity,
                type_name: type_name.clone(),
            });
        }
    }
    if data.prefab != expanded.link {
        buffer.push(WorldCommand::SetPrefabLink {
            entity,
            link: expanded.link.clone(),
        });
    }
}

/// The expanded entities ordered so every parent comes before its children.
fn parents_first(entities: &[ExpandedEntity]) -> Vec<&ExpandedEntity> {
    let parents: HashMap<&SceneEntityId, Option<&SceneEntityId>> = entities
        .iter()
        .map(|expanded| (&expanded.entity.id, expanded.entity.parent.as_ref()))
        .collect();
    let depth = |expanded: &ExpandedEntity| {
        let mut depth = 0_usize;
        let mut cursor = expanded.entity.parent.as_ref();
        while let Some(parent) = cursor.filter(|parent| parents.contains_key(parent)) {
            depth += 1;
            cursor = parents[parent];
        }
        depth
    };
    let mut ordered: Vec<&ExpandedEntity> = entities.iter().collect();
    ordered.sort_by_key(|expanded| depth(expanded));
    ordered
}
