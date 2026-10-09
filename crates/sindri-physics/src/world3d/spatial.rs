//! Query-only per-piece BVH, independent of the solver's synchronization timing.

use std::collections::{HashMap, HashSet};

use rapier3d::parry::bounding_volume::{Aabb, BoundingVolume};
use rapier3d::parry::partitioning::Bvh;
use rapier3d::parry::query::{Ray, RayCast};
use rapier3d::parry::shape::Shape;
use sindri_core::EntityId;

use super::{PhysicsWorld3d, r3};

#[derive(Clone, Copy)]
pub(super) struct QueryPiece {
    pub entity: EntityId,
    pub order: usize,
    pub handle: r3::ColliderHandle,
}

#[derive(Clone, Default)]
pub(super) struct SpatialIndex {
    tree: Bvh,
    pieces: HashMap<u32, QueryPiece>,
    // Keep overflow fallback separate so normal queries never scan all pieces.
    unbounded: HashSet<u32>,
    moving: HashSet<EntityId>,
}

impl SpatialIndex {
    fn update(&mut self, piece: QueryPiece, bounds: Aabb) {
        let key = piece.handle.into_raw_parts().0;
        let bounds = padded(bounds);
        if let Some(bounds) = bounds {
            self.unbounded.remove(&key);
            let _ = self
                .tree
                .reinsert_or_update_with_change_detection(bounds, key, 0.0);
        } else {
            self.unbounded.insert(key);
            self.tree.remove(key);
        }
        self.pieces.insert(key, piece);
    }

    pub(super) fn remove_body(&mut self, entity: EntityId) {
        self.moving.remove(&entity);
    }

    pub(super) fn remove(&mut self, handle: r3::ColliderHandle) {
        let key = handle.into_raw_parts().0;
        self.tree.remove(key);
        self.pieces.remove(&key);
        self.unbounded.remove(&key);
    }

    pub(super) fn ray(&self, ray: &Ray, distance: f32) -> Vec<QueryPiece> {
        self.ordered(
            self.tree
                .leaves(|node| node.aabb().cast_local_ray(ray, distance, true).is_some()),
        )
    }

    pub(super) fn area(&self, bounds: Aabb) -> Vec<QueryPiece> {
        let Some(bounds) = padded(bounds) else {
            let mut pieces: Vec<_> = self.pieces.values().copied().collect();
            pieces.sort_unstable_by_key(|piece| (piece.entity, piece.order));
            return pieces;
        };
        self.ordered(self.tree.intersect_aabb(&bounds))
    }

    fn ordered(&self, keys: impl Iterator<Item = u32>) -> Vec<QueryPiece> {
        let mut pieces: Vec<_> = keys.map(|key| self.pieces[&key]).collect();
        pieces.extend(self.unbounded.iter().map(|key| self.pieces[key]));
        pieces.sort_unstable_by_key(|piece| (piece.entity, piece.order));
        pieces
    }
}

impl PhysicsWorld3d {
    pub(super) fn visit_candidates(
        &self,
        candidates: impl IntoIterator<Item = QueryPiece>,
        filter: crate::RaycastFilter3d,
        include: &mut impl FnMut(EntityId) -> bool,
        mut visit: impl FnMut(EntityId, &dyn Shape, r3::Pose),
    ) {
        let mut last_entity = None;
        let mut accepted = false;
        for candidate in candidates {
            let entity = candidate.entity;
            if last_entity != Some(entity) {
                accepted = filter.exclude != Some(entity) && include(entity);
                last_entity = Some(entity);
            }
            if !accepted {
                continue;
            }
            let body = &self.backend.bodies[self.bodies[&entity].body];
            let collider = &self.backend.colliders[candidate.handle];
            if !collider.is_enabled()
                || (!filter.include_sensors && collider.is_sensor())
                || collider.collision_groups().memberships.bits() & filter.mask == 0
            {
                continue;
            }
            let pose = collider
                .position_wrt_parent()
                .map_or(*collider.position(), |local| *body.position() * *local);
            visit(entity, collider.shape(), pose);
        }
    }

    pub(super) fn index_body(&mut self, entity: EntityId) {
        let record = &self.bodies[&entity];
        if record.kind != crate::RigidBodyKind::Static {
            self.spatial.moving.insert(entity);
        }
        let body = &self.backend.bodies[record.body];
        for (order, &handle) in record.colliders.iter().enumerate() {
            let collider = &self.backend.colliders[handle];
            let pose = collider
                .position_wrt_parent()
                .map_or(*collider.position(), |local| *body.position() * *local);
            self.spatial.update(
                QueryPiece {
                    entity,
                    order,
                    handle,
                },
                collider.shape().compute_aabb(&pose),
            );
        }
    }

    pub(super) fn index_simulated_bodies(&mut self) {
        // Static geometry changes only through insertion/removal/teleport.
        // Sleeping dynamic bodies still cost a bounds check, but not a rebuild.
        let moving: Vec<_> = self.spatial.moving.iter().copied().collect();
        for entity in moving {
            self.index_body(entity);
        }
    }
}

/// Outward rounding protects touching hits at rotated/large-coordinate bounds.
/// If the expansion overflows, use exact geometry on the conservative scan path.
fn padded(bounds: Aabb) -> Option<Aabb> {
    let magnitude = bounds
        .mins
        .abs()
        .max(bounds.maxs.abs())
        .max_element()
        .max(1.0);
    let bounds = bounds.loosened(magnitude * (4.0 * f32::EPSILON));
    (bounds.mins.is_finite() && bounds.maxs.is_finite()).then_some(bounds)
}

#[cfg(test)]
mod tests;
