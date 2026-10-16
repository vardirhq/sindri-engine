//! Live coefficients on existing primitive, hull and triangle colliders.

use sindri_core::EntityId;

use super::PhysicsWorld3d;
use crate::{PhysicsError, PhysicsMaterial};

impl PhysicsWorld3d {
    /// Copied coefficients in registration order: original primitive pieces,
    /// then hulls, followed by static groups in ascending key and piece order.
    ///
    /// # Errors
    /// Returns an error when the entity is not registered.
    pub fn materials(&self, entity: EntityId) -> Result<Vec<PhysicsMaterial>, PhysicsError> {
        Ok(self
            .record(entity)?
            .colliders
            .iter()
            .map(|handle| {
                let collider = &self.backend.colliders[*handle];
                PhysicsMaterial {
                    friction: collider.friction(),
                    restitution: collider.restitution(),
                }
            })
            .collect())
    }

    /// Validates the whole request before updating any piece. Keeps body and
    /// collider identities, geometry, mass, motion, forces and pending controls.
    /// Changed coefficients wake affected contacts; identical values do nothing.
    /// An existing static owner with no geometry accepts an empty slice.
    ///
    /// # Errors
    /// Invalid coefficients, a missing body or a piece count mismatch leave
    /// every collider and the body's state unchanged.
    pub fn set_materials(
        &mut self,
        entity: EntityId,
        materials: &[PhysicsMaterial],
    ) -> Result<(), PhysicsError> {
        let record = self.record(entity)?.clone();
        if materials.len() != record.colliders.len() {
            return Err(PhysicsError::MaterialPieceCount {
                expected: record.colliders.len(),
                actual: materials.len(),
            });
        }
        for material in materials {
            material.validate()?;
        }
        let mut changed = false;
        for (handle, material) in record.colliders.iter().zip(materials) {
            let collider = &mut self.backend.colliders[*handle];
            let current = PhysicsMaterial {
                friction: collider.friction(),
                restitution: collider.restitution(),
            };
            if current == *material {
                continue;
            }
            collider.set_friction(material.friction);
            collider.set_restitution(material.restitution);
            // Rapier coefficient setters do not flag contact changes. Refresh
            // the unchanged pose to reconsider existing contacts and neighbors.
            collider.set_position(*collider.position());
            self.wake_material_contacts(*handle);
            changed = true;
        }
        if changed {
            self.backend.wake_up(record.body, true);
            self.materials_changed = true;
        }
        Ok(())
    }

    fn wake_material_contacts(&mut self, handle: super::r3::ColliderHandle) {
        // A fixed owner cannot propagate a body's own wake request. Explicitly
        // wake contact neighbors so a resting body uses the new coefficients.
        let neighbors: Vec<_> = self
            .backend
            .narrow_phase
            .contact_pairs_with(handle)
            .filter(|pair| pair.has_any_active_contact())
            .filter_map(|pair| {
                let other = if pair.collider1 == handle {
                    pair.collider2
                } else {
                    pair.collider1
                };
                self.backend.colliders[other].parent()
            })
            .collect();
        for neighbor in neighbors {
            self.backend.wake_up(neighbor, true);
        }
    }
}

#[cfg(test)]
#[path = "materials_tests.rs"]
mod tests;
