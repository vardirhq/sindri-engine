//! Static triangle surfaces use the ordinary entity/group/query lifecycle.

use sindri_core::EntityId;

use super::{
    PhysicsWorld3d, build, r3,
    validation::{finite3, validate_pose},
};
use crate::{PhysicsError, PhysicsPose3d, TriangleMeshCollider3d};

impl PhysicsWorld3d {
    /// Checks all triangle pieces, the owner pose and static ownership without
    /// mutation, including incompatible pending motion on an absent owner.
    ///
    /// # Errors
    /// Invalid local/world poses or coefficients name the failing input.
    /// A nonstatic owner or incompatible pending control is rejected.
    pub fn validate_static_mesh_group(
        &self,
        entity: EntityId,
        pose: PhysicsPose3d,
        pieces: &[TriangleMeshCollider3d],
    ) -> Result<(), PhysicsError> {
        validate_pose(pose)?;
        for (index, piece) in pieces.iter().enumerate() {
            validate_piece(piece, pose).map_err(|reason| PhysicsError::ColliderPiece {
                index,
                reason: Box::new(reason),
            })?;
        }
        self.validate_static_owner(entity, !pieces.is_empty())
    }

    /// Replaces a keyed static group with shared triangle surfaces. It shares
    /// keys, ownership, ordering, replacement and removal with primitive groups.
    /// The pose moves the whole owner; piece poses remain local. Geometry is
    /// visible to rays, overlaps, casts and character queries immediately.
    /// An empty group removes the key and never creates an absent owner.
    ///
    /// # Errors
    /// Has the atomic validation contract of [`Self::validate_static_mesh_group`].
    pub fn replace_static_mesh_group(
        &mut self,
        entity: EntityId,
        key: u64,
        pose: PhysicsPose3d,
        pieces: &[TriangleMeshCollider3d],
    ) -> Result<(), PhysicsError> {
        self.validate_static_mesh_group(entity, pose, pieces)?;
        let prepared = pieces.iter().map(|piece| collider(entity, piece)).collect();
        self.replace_prepared_static_group(entity, key, pose, prepared)
    }
}

fn validate_piece(
    piece: &TriangleMeshCollider3d,
    owner: PhysicsPose3d,
) -> Result<(), PhysicsError> {
    validate_pose(piece.pose)?;
    piece.material.validate()?;
    let placed = build::pose(owner) * build::pose(piece.pose);
    let bounds = piece.geometry.shape.compute_aabb(&placed);
    finite3("mesh_world_bounds", bounds.mins.to_array())?;
    finite3("mesh_world_bounds", bounds.maxs.to_array())
}

fn collider(entity: EntityId, piece: &TriangleMeshCollider3d) -> r3::ColliderBuilder {
    build::properties(
        entity,
        r3::ColliderBuilder::new(piece.geometry.shape.clone()),
        piece.pose,
        piece.sensor,
        piece.layers,
        piece.material,
    )
}

#[cfg(test)]
#[path = "triangle_mesh_tests.rs"]
mod tests;
