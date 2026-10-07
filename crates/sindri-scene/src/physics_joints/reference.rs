//! Physics uses the shared authored reference rules, requiring active endpoints.
use sindri_core::{EntityId, World};

pub(super) fn resolve(world: &World, owner: EntityId, target: &str) -> Option<EntityId> {
    world
        .resolve_entity_reference(owner, target)
        .filter(|&entity| world.is_active(entity))
}
