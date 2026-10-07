//! Entity references shared by scene consumers and authoring tools.
use crate::{EntityId, SceneEntityId, World};

impl World {
    /// Finds an authored string that resolves to `target` in the owner's scope.
    ///
    /// Uses canonical local paths for runtime prefabs and stable IDs otherwise.
    /// Returns `None` for stale/unstable or out-of-scope targets. Activity does not
    /// affect whether a reference can be authored.
    #[must_use]
    pub fn entity_reference(&self, owner: EntityId, target: EntityId) -> Option<String> {
        let data = self.get(target)?;
        let id = if self.get(owner)?.prefab_identity.is_some() {
            &data.prefab_identity.as_ref()?.path
        } else {
            data.source_id.as_ref()?
        };
        let mut reference = id.as_str();
        loop {
            if self.resolve_entity_reference(owner, reference) == Some(target) {
                return Some(reference.to_owned());
            }
            let (_, suffix) = reference.split_once('/')?;
            reference = suffix;
        }
    }

    /// Resolves an authored entity string within the owner's scene or prefab.
    ///
    /// Qualified scene IDs precede relative namespaces; canonical IDs precede
    /// prefab aliases. Runtime spawns retain their own local scope. Missing or
    /// stale owners/targets return `None`, and loaded scenes never cross roots.
    /// Inactive targets still resolve: consumers decide whether they can use them.
    #[must_use]
    pub fn resolve_entity_reference(&self, owner: EntityId, target: &str) -> Option<EntityId> {
        let world = self;
        if target.is_empty() {
            return None;
        }
        if world.get(owner)?.prefab_identity.is_some() {
            return world.prefab_entity(owner, target);
        }
        let within = boundary(world, owner);
        let usable = |entity| boundary(world, entity) == within;
        // A qualified ID is relative to its loaded scene's namespace.
        if target.contains('/') {
            let namespace = within.and_then(|root| world.get(root)?.scene_namespace.as_deref());
            let qualified = namespace
                .filter(|name| !name.is_empty())
                .map_or_else(|| target.to_owned(), |name| format!("{name}/{target}"));
            if let Some(entity) = world.entity_for_source_id(&SceneEntityId::new(qualified).ok()?) {
                return usable(entity).then_some(entity);
            }
        }
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
}

/// Loaded scenes have an anonymous root; raw scenes share the unparented scope.
fn boundary(world: &World, mut entity: EntityId) -> Option<EntityId> {
    while let Some(parent) = world.get(entity)?.parent {
        entity = parent;
    }
    world.get(entity)?.source_id.is_none().then_some(entity)
}
