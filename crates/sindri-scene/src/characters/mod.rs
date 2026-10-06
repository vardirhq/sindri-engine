//! Fixed-step scene ownership for reusable character collision movement.
mod component;
mod movement;
mod requests;
#[cfg(test)]
mod tests;

use crate::{PhysicsSyncError, RigidBody2dComponent};
use sindri_core::{ComponentSchemaRegistry, EntityId, World};
use sindri_physics::{Collider2d, GroundedSlideMotion2d, PlatformSupport2d};
use std::collections::BTreeMap;

pub use component::Character2dComponent;
pub use requests::CharacterRequests2d;

#[derive(Clone, Copy, PartialEq)]
pub(super) struct Character {
    settings: Character2dComponent,
    probe: Collider2d,
    parent: Option<EntityId>,
}

#[derive(Default)]
pub(super) struct CharacterState {
    support: Option<PlatformSupport2d>,
    drop_seconds: f32,
    motion: Option<GroundedSlideMotion2d>,
    last_pose: Option<sindri_physics::PhysicsPose2d>,
}

/// Borrowed results from the last controller pass; copying the view does not copy motions.
#[derive(Clone, Copy)]
pub struct CharacterMotions2d<'a> {
    states: &'a BTreeMap<EntityId, CharacterState>,
}

impl<'a> CharacterMotions2d<'a> {
    /// A cached result; callers must still check scene activity during a script pass.
    #[must_use]
    pub fn get(self, entity: EntityId) -> Option<&'a GroundedSlideMotion2d> {
        self.states.get(&entity)?.motion.as_ref()
    }
}

#[derive(Default)]
pub(crate) struct SceneCharacters2d {
    authored: BTreeMap<EntityId, Character>,
    states: BTreeMap<EntityId, CharacterState>,
    pub requests: CharacterRequests2d,
}

impl SceneCharacters2d {
    pub fn for_scripts(&mut self) -> (&mut CharacterRequests2d, CharacterMotions2d<'_>) {
        (
            &mut self.requests,
            CharacterMotions2d {
                states: &self.states,
            },
        )
    }

    pub fn contains(&self, entity: EntityId) -> bool {
        self.authored.contains_key(&entity)
    }

    pub fn motion(&self, entity: EntityId) -> Option<&GroundedSlideMotion2d> {
        self.states.get(&entity)?.motion.as_ref()
    }

    /// Structural edits/teleports invalidate both an actor and its riders.
    /// A queued input remains valid for a rebuilt actor in this same step.
    pub fn invalidate(&mut self, entity: EntityId) {
        self.states.remove(&entity);
        for state in self.states.values_mut() {
            if state
                .support
                .is_some_and(|support| support.entity == entity)
            {
                state.support = None;
                state.motion = None;
            }
        }
    }

    pub fn prepare(
        &mut self,
        world: &World,
        components: &ComponentSchemaRegistry,
        pieces: &BTreeMap<EntityId, Vec<Collider2d>>,
    ) -> Result<(), PhysicsSyncError> {
        let mut authored = BTreeMap::new();
        for (entity, settings) in components.query::<Character2dComponent>(world)? {
            if components
                .get::<RigidBody2dComponent>(world, entity)?
                .is_some()
            {
                return Err(PhysicsSyncError::InvalidCharacter(
                    entity,
                    "a character owns its body; remove the rigid body component",
                ));
            }
            if world.world_transform(entity).is_none() {
                return Err(PhysicsSyncError::InvalidCharacter(
                    entity,
                    "a character requires a transform",
                ));
            }
            settings.options().validate()?;
            let solids: Vec<_> = pieces
                .get(&entity)
                .into_iter()
                .flatten()
                .filter(|piece| !piece.sensor)
                .copied()
                .collect();
            let [probe] = solids.as_slice() else {
                return Err(PhysicsSyncError::InvalidCharacter(
                    entity,
                    "a character requires exactly one solid collider piece",
                ));
            };
            let character = Character {
                settings,
                probe: *probe,
                parent: world.get(entity).and_then(|data| data.parent),
            };
            authored.insert(entity, character);
        }
        // Validate all controllers before changing the previous runtime state.
        let changed: Vec<_> = self
            .authored
            .keys()
            .chain(authored.keys())
            .filter(|entity| self.authored.get(entity) != authored.get(entity))
            .copied()
            .collect();
        for entity in changed {
            self.invalidate(entity);
        }
        self.authored = authored;
        self.states
            .retain(|entity, _| self.authored.contains_key(entity));
        self.requests
            .motion
            .retain(|entity, _| self.authored.contains_key(entity));
        self.requests
            .drop
            .retain(|entity, _| self.authored.contains_key(entity));
        Ok(())
    }
}
