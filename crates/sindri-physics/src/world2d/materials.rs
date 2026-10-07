//! Live coefficient updates preserve bodies and joints.

use sindri_core::EntityId;

use super::PhysicsWorld2d;
use crate::{PhysicsError, PhysicsMaterial};

impl PhysicsWorld2d {
    /// Coefficients in the same piece order used at registration.
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

    /// Validates every coefficient before updating any piece. Wakes affected
    /// contacts without replacing the body, its velocity, forces or joints.
    ///
    /// # Errors
    /// Returns an error for invalid coefficients, a missing body or a piece count mismatch.
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
        for (handle, material) in record.colliders.iter().zip(materials) {
            let collider = &mut self.backend.colliders[*handle];
            collider.set_friction(material.friction);
            collider.set_restitution(material.restitution);
            // Coefficient setters do not flag changes in Rapier. Refresh the
            // pose so sleeping neighbors and existing contacts are reconsidered.
            collider.set_position(*collider.position());
        }
        self.backend.bodies[record.body].wake_up(true);
        Ok(())
    }
}
