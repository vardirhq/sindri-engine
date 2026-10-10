//! Scene ownership around the read-only 3D character movement primitive.
#[cfg(test)]
mod carry_tests;
mod component;
mod movement;
mod requests;
mod residency;
#[cfg(test)]
mod tests;

pub use component::Character3dComponent;
pub use requests::CharacterRequests3d;

use crate::{Collider3dComponent, PhysicsSyncError, RigidBody3dComponent};
use sindri_core::{ComponentSchemaRegistry, EntityId, World};
use sindri_physics::{Collider3d, GroundedCharacterMotion3d, PlatformSupport3d};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Character {
    pub settings: Character3dComponent,
    pub probe: Collider3d,
    parent: Option<EntityId>,
}

pub(crate) type CharacterPlan = BTreeMap<EntityId, Character>;

/// Borrowed results of the last completed movement pass.
#[derive(Clone, Copy)]
pub struct CharacterMotions3d<'a> {
    states: &'a BTreeMap<EntityId, GroundedCharacterMotion3d>,
}

impl<'a> CharacterMotions3d<'a> {
    /// Callers must also check scene activity during a script pass.
    #[must_use]
    pub fn get(self, entity: EntityId) -> Option<&'a GroundedCharacterMotion3d> {
        self.states.get(&entity)
    }
}

#[derive(Clone, Default)]
pub(crate) struct SceneCharacters3d {
    authored: CharacterPlan,
    states: BTreeMap<EntityId, GroundedCharacterMotion3d>,
    supports: BTreeMap<EntityId, PlatformSupport3d>,
    pub requests: CharacterRequests3d,
}

impl SceneCharacters3d {
    pub fn for_scripts(&mut self) -> (&mut CharacterRequests3d, CharacterMotions3d<'_>) {
        (
            &mut self.requests,
            CharacterMotions3d {
                states: &self.states,
            },
        )
    }

    pub fn motion(&self, entity: EntityId) -> Option<&GroundedCharacterMotion3d> {
        self.states.get(&entity)
    }

    pub fn invalidate(&mut self, entity: EntityId) {
        self.states.retain(|&actor, motion| {
            actor != entity
                && motion.ground.hit.is_none_or(|hit| hit.entity != entity)
                && motion
                    .platform
                    .as_ref()
                    .is_none_or(|carry| carry.entity != entity)
        });
        self.supports
            .retain(|&actor, support| actor != entity && support.entity != entity);
    }

    /// Validate every controller before lifecycle, gravity or queues change.
    pub fn plan(
        world: &World,
        components: &ComponentSchemaRegistry,
    ) -> Result<CharacterPlan, PhysicsSyncError> {
        let mut plan = BTreeMap::new();
        for (entity, settings) in components.query::<Character3dComponent>(world)? {
            if components
                .get::<RigidBody3dComponent>(world, entity)?
                .is_some()
            {
                return Err(PhysicsSyncError::InvalidCharacter(
                    entity,
                    "a character owns its body; remove the rigid body component",
                ));
            }
            let transform =
                world
                    .world_transform(entity)
                    .ok_or(PhysicsSyncError::InvalidCharacter(
                        entity,
                        "a character requires a transform",
                    ))?;
            settings.movement.validate()?;
            let collider = components
                .get::<Collider3dComponent>(world, entity)?
                .ok_or(PhysicsSyncError::InvalidCharacter(
                    entity,
                    "a character requires exactly one solid collider piece",
                ))?;
            let pieces = collider
                .scaled(transform.scale)
                .map_err(|error| PhysicsSyncError::ColliderScale3d(entity, error))?;
            let mut solids = pieces.into_iter().filter(|piece| !piece.sensor);
            let probe = solids.next().ok_or(PhysicsSyncError::InvalidCharacter(
                entity,
                "a character requires exactly one solid collider piece",
            ))?;
            if solids.next().is_some() {
                return Err(PhysicsSyncError::InvalidCharacter(
                    entity,
                    "a character requires exactly one solid collider piece",
                ));
            }
            plan.insert(
                entity,
                Character {
                    settings,
                    probe,
                    parent: world.get(entity).and_then(|data| data.parent),
                },
            );
        }
        Ok(plan)
    }

    pub fn commit(&mut self, plan: CharacterPlan) {
        self.states.retain(|entity, _| {
            self.authored.get(entity) == plan.get(entity) && plan.contains_key(entity)
        });
        self.requests
            .motion
            .retain(|entity, _| plan.contains_key(entity));
        self.authored = plan;
    }
}
