//! Scene scope and prefab-instance endpoint resolution.
use sindri_core::{EntityId, SceneEntityId, World};

/// Searches the owner's ID namespace before the containing scene. An inactive
/// local target never falls through to a similarly named external entity.
pub(super) fn resolve(world: &World, owner: EntityId, target: &str) -> Option<EntityId> {
    if target.is_empty() {
        return None;
    }
    if world.get(owner)?.prefab_identity.is_some() {
        return world
            .prefab_entity(owner, target)
            .filter(|&entity| world.is_active(entity));
    }
    let within = boundary(world, owner);
    let usable = |entity| boundary(world, entity) == within && world.is_active(entity);
    let find = |key: &SceneEntityId| {
        world.entity_for_source_id(key).or_else(|| {
            world.entities().find_map(|(entity, data)| {
                (boundary(world, entity) == within
                    && data
                        .prefab
                        .as_ref()
                        .is_some_and(|link| link.aliases.contains(key)))
                .then_some(entity)
            })
        })
    };
    let mut namespace = world
        .get(owner)?
        .source_id
        .as_ref()
        .map_or("", SceneEntityId::as_str);
    while let Some((prefix, _)) = namespace.rsplit_once('/') {
        let key = SceneEntityId::new(format!("{prefix}/{target}")).ok()?;
        if let Some(entity) = find(&key) {
            return usable(entity).then_some(entity);
        }
        namespace = prefix;
    }
    let entity = find(&SceneEntityId::new(target).ok()?)?;
    usable(entity).then_some(entity)
}

/// Loaded scenes have an anonymous root; raw scenes share the unparented scope.
fn boundary(world: &World, mut entity: EntityId) -> Option<EntityId> {
    while let Some(parent) = world.get(entity)?.parent {
        entity = parent;
    }
    world.get(entity)?.source_id.is_none().then_some(entity)
}
