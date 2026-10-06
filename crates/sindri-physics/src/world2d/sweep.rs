//! Area checks and swept shapes: what a shape placed somewhere overlaps, and
//! what it would first touch moved along a line.
//!
//! Both answer from current body poses by scanning every registered piece, as
//! the raycast does, so they work before the first step and need no
//! broad-phase index kept fresh between synchronizations.

use rapier2d::parry::query::{self, ShapeCastOptions};
use rapier2d::parry::shape::Shape;

use super::{PhysicsWorld2d, r2};
use crate::validate::{finite, finite2, non_negative, positive};
use crate::{ColliderShape2d, PhysicsError, PhysicsPose2d, RaycastFilter2d};
use sindri_core::EntityId;

/// The first thing a swept shape touches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeHit2d {
    pub entity: EntityId,
    /// Where the two touch, in world coordinates.
    pub point: [f32; 2],
    /// The touched surface's outward normal; zero when the shape starts
    /// already overlapping it.
    pub normal: [f32; 2],
    /// How far the shape travelled before touching, in world units.
    pub distance: f32,
}

impl PhysicsWorld2d {
    /// [`Self::overlap_where`] with nothing excluded but what `filter` says.
    ///
    /// # Errors
    /// As [`Self::overlap_where`].
    pub fn overlap(
        &self,
        shape: ColliderShape2d,
        pose: PhysicsPose2d,
        filter: RaycastFilter2d,
    ) -> Result<Vec<EntityId>, PhysicsError> {
        self.overlap_where(shape, pose, filter, |_| true)
    }

    /// [`Self::shape_cast_where`] with nothing excluded but what `filter` says.
    ///
    /// # Errors
    /// As [`Self::shape_cast_where`].
    pub fn shape_cast(
        &self,
        shape: ColliderShape2d,
        pose: PhysicsPose2d,
        direction: [f32; 2],
        max_distance: f32,
        filter: RaycastFilter2d,
    ) -> Result<Option<ShapeHit2d>, PhysicsError> {
        self.shape_cast_where(shape, pose, direction, max_distance, filter, |_| true)
    }

    /// Every entity with a piece overlapping `shape` placed at `pose`, each
    /// once, in handle order.
    ///
    /// # Errors
    /// Rejects a non-finite pose and a shape with a non-positive size.
    pub fn overlap_where(
        &self,
        shape: ColliderShape2d,
        pose: PhysicsPose2d,
        filter: RaycastFilter2d,
        mut include: impl FnMut(EntityId) -> bool,
    ) -> Result<Vec<EntityId>, PhysicsError> {
        let probe = query_shape(shape)?;
        let at = query_pose(pose)?;
        let mut found = Vec::new();
        self.each_piece(filter, &mut include, |entity, piece, piece_pose| {
            // Pieces arrive grouped by entity, in handle order, so a second
            // overlapping piece of the same entity is always the last found.
            if found.last() != Some(&entity)
                && query::intersection_test(&at, probe.as_ref(), &piece_pose, piece)
                    .is_ok_and(|intersection| intersection.intersecting)
            {
                found.push(entity);
            }
        });
        Ok(found)
    }

    /// Moves `shape` from `pose` along `direction` for up to `max_distance`,
    /// without rotating it, and reports the first piece it touches.
    ///
    /// Starting inside a piece gives a zero-distance hit with a zero normal,
    /// as the raycast does. Exact ties prefer the smaller entity handle.
    ///
    /// # Errors
    /// Rejects non-finite values, a negative distance, a zero direction and a
    /// shape with a non-positive size.
    pub fn shape_cast_where(
        &self,
        shape: ColliderShape2d,
        pose: PhysicsPose2d,
        direction: [f32; 2],
        max_distance: f32,
        filter: RaycastFilter2d,
        mut include: impl FnMut(EntityId) -> bool,
    ) -> Result<Option<ShapeHit2d>, PhysicsError> {
        let probe = query_shape(shape)?;
        let at = query_pose(pose)?;
        finite2("cast_direction", direction)?;
        non_negative("cast_distance", max_distance)?;
        let length = direction[0].hypot(direction[1]);
        if length <= 0.0 || !length.is_finite() {
            return Err(PhysicsError::NonPositive("cast_direction_length"));
        }
        let velocity = r2::Vector::new(direction[0] / length, direction[1] / length);
        let options = ShapeCastOptions {
            max_time_of_impact: max_distance,
            stop_at_penetration: true,
            compute_impact_geometry_on_penetration: false,
            ..ShapeCastOptions::default()
        };
        let mut closest: Option<ShapeHit2d> = None;
        self.each_piece(filter, &mut include, |entity, piece, piece_pose| {
            let Ok(Some(hit)) = query::cast_shapes(
                &at,
                velocity,
                probe.as_ref(),
                &piece_pose,
                r2::Vector::ZERO,
                piece,
                options,
            ) else {
                return;
            };
            let distance = hit.time_of_impact;
            if closest.as_ref().is_some_and(|old| {
                distance
                    .total_cmp(&old.distance)
                    .then(entity.cmp(&old.entity))
                    .is_ge()
            }) {
                return;
            }
            let (point, normal) = if distance <= 0.0 {
                (at.translation, r2::Vector::ZERO)
            } else {
                (
                    piece_pose.transform_point(hit.witness2),
                    piece_pose.rotation.transform_vector(hit.normal2),
                )
            };
            closest = Some(ShapeHit2d {
                entity,
                point: [point.x, point.y],
                normal: [normal.x, normal.y],
                distance,
            });
        });
        Ok(closest)
    }

    /// Calls `visit` with every enabled piece the filter lets through, posed
    /// in the world.
    pub(super) fn each_piece(
        &self,
        filter: RaycastFilter2d,
        include: &mut impl FnMut(EntityId) -> bool,
        mut visit: impl FnMut(EntityId, &dyn Shape, r2::Pose),
    ) {
        self.each_piece_policy(filter, include, |entity, piece, pose, _| {
            visit(entity, piece, pose);
        });
    }

    pub(super) fn each_piece_policy(
        &self,
        filter: RaycastFilter2d,
        include: &mut impl FnMut(EntityId) -> bool,
        mut visit: impl FnMut(EntityId, &dyn Shape, r2::Pose, Option<crate::OneWay2d>),
    ) {
        let mut entities: Vec<&EntityId> = self.bodies.keys().collect();
        entities.sort_unstable();
        for &entity in entities {
            if filter.exclude == Some(entity) || !include(entity) {
                continue;
            }
            let record = &self.bodies[&entity];
            let body = &self.backend.bodies[record.body];
            for &handle in &record.colliders {
                let collider = &self.backend.colliders[handle];
                if !collider.is_enabled()
                    || (!filter.include_sensors && collider.is_sensor())
                    || collider.collision_groups().memberships.bits() & filter.mask == 0
                {
                    continue;
                }
                let pose = collider
                    .position_wrt_parent()
                    .map_or(*collider.position(), |local| *body.position() * *local);
                visit(
                    entity,
                    collider.shape(),
                    pose,
                    self.one_way.policies.get(&handle).copied(),
                );
            }
        }
    }
}

pub(super) fn query_shape(shape: ColliderShape2d) -> Result<r2::SharedShape, PhysicsError> {
    Ok(match shape {
        ColliderShape2d::Box { half_extents } => {
            positive("query_half_extent", half_extents[0])?;
            positive("query_half_extent", half_extents[1])?;
            r2::SharedShape::cuboid(half_extents[0], half_extents[1])
        }
        ColliderShape2d::Circle { radius } => {
            positive("query_radius", radius)?;
            r2::SharedShape::ball(radius)
        }
        ColliderShape2d::Capsule {
            half_height,
            radius,
        } => {
            non_negative("query_half_height", half_height)?;
            positive("query_radius", radius)?;
            r2::SharedShape::capsule_y(half_height, radius)
        }
    })
}

pub(super) fn query_pose(pose: PhysicsPose2d) -> Result<r2::Pose, PhysicsError> {
    finite2("query_position", pose.position)?;
    finite("query_rotation", pose.rotation)?;
    Ok(r2::Pose::new(
        r2::Vector::new(pose.position[0], pose.position[1]),
        pose.rotation,
    ))
}
