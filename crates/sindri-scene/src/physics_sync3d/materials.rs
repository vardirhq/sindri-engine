//! Resolve coefficients before committing any body, preserving geometry on edits.
use std::collections::BTreeMap;

use sindri_core::{ComponentSchemaRegistry, EntityId, World};
use sindri_physics::PhysicsMaterial;

use super::Authored;
use crate::{PhysicsMaterial3dComponent, PhysicsMaterialSources, PhysicsSyncError};

pub(super) fn apply(
    world: &World,
    components: &ComponentSchemaRegistry,
    sources: &PhysicsMaterialSources,
    plan: &mut BTreeMap<EntityId, Authored>,
) -> Result<(), PhysicsSyncError> {
    for (&entity, authored) in plan {
        let Some(component) = components.get::<PhysicsMaterial3dComponent>(world, entity)? else {
            continue;
        };
        for piece in &mut authored.pieces {
            let resolved = sources.resolve(
                &component.0,
                PhysicsMaterial {
                    friction: piece.friction,
                    restitution: piece.restitution,
                },
            )?;
            piece.friction = resolved.friction;
            piece.restitution = resolved.restitution;
        }
        if let Some(model) = &mut authored.model {
            model.piece.material = sources.resolve(&component.0, model.piece.material)?;
        }
        if let Some(convex) = &mut authored.convex {
            convex.piece.material = sources.resolve(&component.0, convex.piece.material)?;
        }
    }
    Ok(())
}

pub(super) fn same_geometry(previous: &Authored, authored: &Authored) -> bool {
    if previous == authored {
        return true;
    }
    let mut comparable = previous.clone();
    for (before, after) in comparable.pieces.iter_mut().zip(&authored.pieces) {
        before.friction = after.friction;
        before.restitution = after.restitution;
    }
    if let (Some(before), Some(after)) = (&mut comparable.model, &authored.model) {
        before.copy_material_from(after);
    }
    if let (Some(before), Some(after)) = (&mut comparable.convex, &authored.convex) {
        before.copy_material_from(after);
    }
    comparable == *authored
}

pub(super) fn coefficients(authored: &Authored) -> Vec<PhysicsMaterial> {
    authored
        .pieces
        .iter()
        .map(|piece| PhysicsMaterial {
            friction: piece.friction,
            restitution: piece.restitution,
        })
        .chain(authored.model.iter().map(|model| model.piece.material))
        .chain(authored.convex.iter().map(|convex| convex.piece.material))
        .collect()
}
