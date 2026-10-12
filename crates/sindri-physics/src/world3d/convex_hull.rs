//! Hull compounds use ordinary body insertion, controls and queries.

use sindri_core::EntityId;

use super::{
    PhysicsWorld3d, build, r3,
    validation::{finite3, validate_body, validate_colliders, validate_pose},
};
use crate::{Collider3d, ConvexHullCollider3d, PhysicsError, PhysicsPose3d, RigidBody3d};

impl PhysicsWorld3d {
    /// Checks a compound of primitives followed by convex hull pieces. Piece
    /// diagnostics use that combined ordering. The 1,024-piece budget applies
    /// to both lists together. At least one piece is required.
    ///
    /// # Errors
    /// Rejects body/piece values, overflowing bounds or unusable combined mass,
    /// incompatible pending controls before mutation. Existing registration is ignored
    /// so scene drivers can prevalidate a rebuild.
    pub fn validate_hull_body(
        &self,
        entity: EntityId,
        body: RigidBody3d,
        primitives: &[Collider3d],
        hulls: &[ConvexHullCollider3d],
    ) -> Result<(), PhysicsError> {
        self.prepare_hull_body(entity, body, primitives, hulls)
            .map(|_| ())
    }

    /// Inserts shared convex pieces alongside optional primitive colliders.
    /// Supports static, dynamic and both kinematic kinds. Uses the same mass,
    /// pending controls, immediate queries, removal and replay as primitive bodies.
    /// Sensor pieces contribute mass as existing primitive sensors do.
    ///
    /// # Errors
    /// Has the atomic validation contract of [`Self::validate_hull_body`].
    pub fn insert_hull_body(
        &mut self,
        entity: EntityId,
        body: RigidBody3d,
        primitives: &[Collider3d],
        hulls: &[ConvexHullCollider3d],
    ) -> Result<(), PhysicsError> {
        if self.contains(entity) {
            return Err(PhysicsError::EntityAlreadyRegistered(entity));
        }
        let prepared = self.prepare_hull_body(entity, body, primitives, hulls)?;
        self.insert_prepared_body(entity, body, prepared)
    }

    fn prepare_hull_body(
        &self,
        entity: EntityId,
        body: RigidBody3d,
        primitives: &[Collider3d],
        hulls: &[ConvexHullCollider3d],
    ) -> Result<Vec<r3::ColliderBuilder>, PhysicsError> {
        let count = primitives.len().checked_add(hulls.len());
        if count == Some(0) {
            return Err(PhysicsError::NoColliderPieces(entity));
        }
        if count.is_none_or(|count| count > 1_024) {
            return Err(PhysicsError::TooManyColliderPieces {
                actual: count.unwrap_or(usize::MAX),
                limit: 1_024,
            });
        }
        validate_body(body)?;
        validate_colliders(primitives)?;
        self.validate_pending_controls(entity, body.kind)?;
        let mut prepared: Vec<_> = primitives
            .iter()
            .map(|&piece| build::collider(entity, piece))
            .collect();
        for (index, piece) in hulls.iter().enumerate() {
            validate_pose(piece.pose)
                .and_then(|()| piece.material.validate())
                .map_err(|reason| PhysicsError::ColliderPiece {
                    index: primitives.len() + index,
                    reason: Box::new(reason),
                })?;
            prepared.push(build::properties(
                entity,
                r3::ColliderBuilder::new(piece.geometry.shape.clone()),
                piece.pose,
                piece.sensor,
                piece.layers,
                piece.material,
            ));
        }
        validate_geometry(body, &prepared)?;
        Ok(prepared)
    }
}

fn validate_geometry(
    body: RigidBody3d,
    pieces: &[r3::ColliderBuilder],
) -> Result<(), PhysicsError> {
    let owner = build::pose(PhysicsPose3d {
        position: body.position,
        rotation: body.rotation,
    });
    let mut combined = r3::MassProperties::default();
    for (index, builder) in pieces.iter().enumerate() {
        let piece = builder.clone().build();
        let bounds = piece.shape().compute_aabb(&(owner * *piece.position()));
        finite3("hull_body_bounds", bounds.mins.to_array())
            .and_then(|()| finite3("hull_body_bounds", bounds.maxs.to_array()))
            .map_err(|reason| PhysicsError::ColliderPiece {
                index,
                reason: Box::new(reason),
            })?;
        let mass = piece.mass_properties();
        if !crate::convex_hull3d::usable_mass(mass) {
            return Err(PhysicsError::ColliderPiece {
                index,
                reason: Box::new(PhysicsError::InvalidMassProperties),
            });
        }
        combined += mass.transform_by(piece.position());
    }
    if !crate::convex_hull3d::usable_mass(combined) {
        return Err(PhysicsError::InvalidMassProperties);
    }
    Ok(())
}

#[cfg(test)]
#[path = "convex_hull_tests.rs"]
mod tests;
