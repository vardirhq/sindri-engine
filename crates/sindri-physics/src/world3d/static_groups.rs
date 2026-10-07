//! Independently replaceable static geometry under one real entity identity.

use sindri_core::EntityId;

use super::{
    PhysicsWorld3d, build,
    validation::{validate_colliders, validate_pose},
};
use crate::{Collider3d, PhysicsError, PhysicsPose3d, RigidBody3d, RigidBodyKind};

impl PhysicsWorld3d {
    /// Checks a static group replacement without changing runtime state.
    /// Hosts prevalidate all section edits before applying a complete batch.
    ///
    /// # Errors
    /// Has the input/kind/pending-control contract of [`Self::replace_static_group`].
    pub fn validate_static_group(
        &self,
        entity: EntityId,
        pose: PhysicsPose3d,
        pieces: &[Collider3d],
    ) -> Result<(), PhysicsError> {
        validate_pose(pose)?;
        validate_colliders(pieces)?;
        if let Some(record) = self.bodies.get(&entity) {
            if record.kind != RigidBodyKind::Static {
                return Err(PhysicsError::WrongBodyKind(
                    entity,
                    "static collider group",
                    record.kind,
                ));
            }
        } else if !pieces.is_empty() {
            self.validate_insertion(
                entity,
                RigidBody3d {
                    kind: RigidBodyKind::Static,
                    position: pose.position,
                    rotation: pose.rotation,
                    ..RigidBody3d::default()
                },
                pieces,
            )?;
        }
        Ok(())
    }

    /// Replaces one keyed static collider group, retaining the owner's other
    /// groups and originally inserted colliders. An absent owner is created as
    /// static on the first nonempty group. Keys are caller-owned runtime IDs,
    /// not entities or serialized scene identities.
    ///
    /// Pose applies to the whole owner (all groups), not just this group. Pieces
    /// retain their local offsets/rotations, layers, sensors and coefficients.
    /// Queries update immediately and retain the owner's entity identity. Ties
    /// use original pieces first, then ascending group key and piece order.
    /// Empty pieces remove this group; an empty absent owner is never created.
    /// An owner with no remaining geometry stays registered until [`Self::remove`].
    ///
    /// # Errors
    /// Invalid pose/pieces or a nonstatic owner fail before any mutation. A new
    /// owner also rejects incompatible pending controls before insertion.
    pub fn replace_static_group(
        &mut self,
        entity: EntityId,
        key: u64,
        pose: PhysicsPose3d,
        pieces: &[Collider3d],
    ) -> Result<(), PhysicsError> {
        self.validate_static_group(entity, pose, pieces)?;
        if !self.contains(entity) {
            if pieces.is_empty() {
                return Ok(());
            }
            self.insert_static_collider(entity, pose, pieces)?;
            // Insertion just validated and created this record. Move its initial
            // handles into group ownership so later replacement releases them.
            let record = self
                .bodies
                .get_mut(&entity)
                .ok_or(PhysicsError::MissingEntity(entity))?;
            record.base_colliders = 0;
            record.groups.insert(key, record.colliders.clone());
            return Ok(());
        }
        self.remove_static_group(entity, key)?;
        let handle = self.record(entity)?.body;
        let handles: Vec<_> = pieces
            .iter()
            .map(|piece| {
                self.backend
                    .insert_collider(build::collider(entity, *piece), Some(handle))
            })
            .collect();
        for &handle in &handles {
            self.collider_entities.insert(handle, entity);
        }
        let record = self
            .bodies
            .get_mut(&entity)
            .ok_or(PhysicsError::MissingEntity(entity))?;
        if !handles.is_empty() {
            record.groups.insert(key, handles);
        }
        record.colliders.truncate(record.base_colliders);
        record
            .colliders
            .extend(record.groups.values().flatten().copied());
        self.move_to(entity, pose)?;
        Ok(())
    }

    /// Releases just one keyed group and updates queries immediately. Keeps the
    /// static body, pose, original colliders and all other groups intact.
    /// Returns false for an unknown group on an existing static owner.
    ///
    /// # Errors
    /// Missing or nonstatic owners fail without mutation.
    pub fn remove_static_group(
        &mut self,
        entity: EntityId,
        key: u64,
    ) -> Result<bool, PhysicsError> {
        let record = self.record(entity)?;
        if record.kind != RigidBodyKind::Static {
            return Err(PhysicsError::WrongBodyKind(
                entity,
                "remove static group",
                record.kind,
            ));
        }
        let record = self
            .bodies
            .get_mut(&entity)
            .ok_or(PhysicsError::MissingEntity(entity))?;
        let Some(handles) = record.groups.remove(&key) else {
            return Ok(false);
        };
        record.colliders.truncate(record.base_colliders);
        record
            .colliders
            .extend(record.groups.values().flatten().copied());
        for handle in handles {
            self.spatial.remove(handle);
            self.collider_entities.remove(&handle);
            let _ = self.backend.remove_collider(handle);
        }
        self.index_body(entity);
        Ok(true)
    }
}

#[cfg(test)]
#[path = "static_groups_tests.rs"]
mod tests;
