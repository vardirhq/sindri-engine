//! Decoded components, kept until their entity changes.
//!
//! A component is stored as JSON, which is what makes a scene a file anyone
//! can read and a payload any tool can edit. Reading one is a decode, and a
//! system that asks for every collider every step decodes every collider
//! every step: on the platformer that was a quarter of the step, spent
//! turning the same unchanged text into the same values sixty times a second.
//!
//! An entity's revision changes whenever anything about it does
//! ([`World::revision`]), so a value decoded at one revision is still the
//! value at that revision. This keeps each decode with the revision it was
//! made at and decodes again only when the revision moved. What is cached is
//! only the decode: whether an entity is active depends on its ancestors as
//! well, so that is asked every time, as [`ComponentSchemaRegistry::query`]
//! asks it.

use std::collections::BTreeMap;

use super::{ComponentRegistryError, ComponentSchemaRegistry, SceneComponent};
use crate::{EntityId, World};

/// One component type's decodes, by entity.
#[derive(Clone, Debug)]
pub struct Decoded<T> {
    held: BTreeMap<EntityId, (u64, Option<T>)>,
}

impl<T> Default for Decoded<T> {
    fn default() -> Self {
        Self {
            held: BTreeMap::new(),
        }
    }
}

impl<T: SceneComponent> Decoded<T> {
    /// What [`ComponentSchemaRegistry::get`] answers, decoding only when the
    /// entity changed since it was last asked about.
    ///
    /// # Errors
    /// As [`ComponentSchemaRegistry::get`].
    pub fn get(
        &mut self,
        registry: &ComponentSchemaRegistry,
        world: &World,
        entity: EntityId,
    ) -> Result<Option<&T>, ComponentRegistryError> {
        if !self.refresh(registry, world, entity)? {
            return Ok(None);
        }
        Ok(self.held.get(&entity).and_then(|(_, value)| value.as_ref()))
    }

    /// What [`ComponentSchemaRegistry::query`] answers, decoding only the
    /// entities that changed, and forgetting the ones that are gone.
    ///
    /// # Errors
    /// As [`ComponentSchemaRegistry::query`].
    pub fn query(
        &mut self,
        registry: &ComponentSchemaRegistry,
        world: &World,
    ) -> Result<Vec<(EntityId, &T)>, ComponentRegistryError> {
        let mut carrying = Vec::new();
        for (entity, data) in world.entities() {
            if data.components.contains_key(T::TYPE_NAME)
                && self.refresh(registry, world, entity)?
            {
                carrying.push(entity);
            }
        }
        if self.held.len() > carrying.len() * 2 + 64 {
            self.held.retain(|entity, _| world.contains(*entity));
        }
        Ok(carrying
            .into_iter()
            .filter_map(|entity| {
                let (_, value) = self.held.get(&entity)?;
                value.as_ref().map(|value| (entity, value))
            })
            .collect())
    }

    /// Brings the held decode of `entity` up to its revision, answering
    /// whether it is active, so there is anything to answer for.
    fn refresh(
        &mut self,
        registry: &ComponentSchemaRegistry,
        world: &World,
        entity: EntityId,
    ) -> Result<bool, ComponentRegistryError> {
        let Some(revision) = world.revision(entity) else {
            self.held.remove(&entity);
            return Ok(false);
        };
        if !world.is_active(entity) {
            return Ok(false);
        }
        if self
            .held
            .get(&entity)
            .is_none_or(|(held, _)| *held != revision)
        {
            let value = registry.get::<T>(world, entity)?;
            self.held.insert(entity, (revision, value));
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;
    use crate::{EntityData, Transform3D};

    #[derive(Clone, Debug, Deserialize, PartialEq)]
    struct Speed {
        value: f32,
    }

    impl SceneComponent for Speed {
        const TYPE_NAME: &'static str = "test.speed";
    }

    fn registry() -> ComponentSchemaRegistry {
        let mut registry = ComponentSchemaRegistry::default();
        registry.register::<Speed>("Speed").unwrap();
        registry
    }

    #[test]
    fn a_changed_entity_is_decoded_again_and_an_unchanged_one_is_not() {
        let registry = registry();
        let mut world = World::default();
        let entity = world.spawn(EntityData {
            components: [(Speed::TYPE_NAME.to_owned(), json!({ "value": 1.0 }))].into(),
            ..EntityData::default()
        });
        let mut decoded = Decoded::<Speed>::default();
        assert_eq!(
            decoded.query(&registry, &world).unwrap(),
            [(entity, &Speed { value: 1.0 })]
        );
        let held = decoded.held.get(&entity).unwrap().0;
        decoded.query(&registry, &world).unwrap();
        assert_eq!(decoded.held.get(&entity).unwrap().0, held, "kept");

        world
            .get_mut(entity)
            .unwrap()
            .components
            .insert(Speed::TYPE_NAME.to_owned(), json!({ "value": 2.0 }));
        assert_eq!(
            decoded.get(&registry, &world, entity).unwrap(),
            Some(&Speed { value: 2.0 }),
            "the edit is seen"
        );
        world.get_mut(entity).unwrap().transform_3d = Some(Transform3D::default());
        assert_eq!(
            decoded.get(&registry, &world, entity).unwrap(),
            Some(&Speed { value: 2.0 })
        );
    }

    #[test]
    fn a_switched_off_entity_is_not_answered_for() {
        let registry = registry();
        let mut world = World::default();
        let entity = world.spawn(EntityData {
            components: [(Speed::TYPE_NAME.to_owned(), json!({ "value": 1.0 }))].into(),
            disabled: true,
            ..EntityData::default()
        });
        let mut decoded = Decoded::<Speed>::default();
        assert!(decoded.query(&registry, &world).unwrap().is_empty());
        assert_eq!(decoded.get(&registry, &world, entity).unwrap(), None);
    }
}
