//! A read-only geometric primitive; gameplay decides how to apply its result.

use rapier2d::parry::query::{self, ShapeCastOptions};
use rapier2d::parry::shape::Shape;
use sindri_core::EntityId;

use super::slope::SlopeLimit2d;
use super::sweep::{query_pose, query_shape};
use super::{PhysicsWorld2d, r2};
use crate::validate::finite2;
use crate::{
    ColliderShape2d, PhysicsError, PhysicsPose2d, RaycastFilter2d, ShapeHit2d, SlideMotion2d,
    SlideOptions2d,
};

/// Shared inputs keep geometric and grounded movement on one sweep path.
#[derive(Clone, Copy)]
pub(super) struct SlideRequest2d {
    pub shape: ColliderShape2d,
    pub pose: PhysicsPose2d,
    pub displacement: [f32; 2],
    pub options: SlideOptions2d,
    pub filter: RaycastFilter2d,
}

impl PhysicsWorld2d {
    /// Computes swept movement, removing only motion into each touched surface.
    ///
    /// Uses current poses, including inserts and teleports before a physics step.
    /// The probe never rotates. Initial penetration blocks motion; touching or
    /// being inside the skin permits tangential/separating motion. One-way
    /// geometry is solid on both sides here, as in ordinary geometric queries.
    /// This primitive does not supply ground, slope, step or platform policy.
    ///
    /// # Errors
    /// Rejects invalid shapes, poses, displacement, destination and options.
    pub fn move_and_slide(
        &self,
        shape: ColliderShape2d,
        pose: PhysicsPose2d,
        displacement: [f32; 2],
        options: SlideOptions2d,
        filter: RaycastFilter2d,
    ) -> Result<SlideMotion2d, PhysicsError> {
        self.move_and_slide_where(shape, pose, displacement, options, filter, |_| true)
    }

    /// [`Self::move_and_slide`] with an additional whole-entity predicate.
    ///
    /// Hosts can exclude inactive entities without a second movement algorithm.
    /// The predicate must be stable for the duration of the call.
    ///
    /// # Errors
    /// As [`Self::move_and_slide`].
    pub fn move_and_slide_where(
        &self,
        shape: ColliderShape2d,
        pose: PhysicsPose2d,
        displacement: [f32; 2],
        options: SlideOptions2d,
        filter: RaycastFilter2d,
        include: impl FnMut(EntityId) -> bool,
    ) -> Result<SlideMotion2d, PhysicsError> {
        self.slide_with_policy(
            SlideRequest2d {
                shape,
                pose,
                displacement,
                options,
                filter,
            },
            None,
            include,
        )
    }

    pub(super) fn slide_with_policy(
        &self,
        request: SlideRequest2d,
        slope: Option<SlopeLimit2d>,
        mut include: impl FnMut(EntityId) -> bool,
    ) -> Result<SlideMotion2d, PhysicsError> {
        let SlideRequest2d {
            shape,
            pose,
            displacement,
            options,
            filter,
        } = request;
        let probe = query_shape(shape)?;
        let mut at = query_pose(pose)?;
        options.validate()?;
        finite2("slide_displacement", displacement)?;
        let wanted = r2::Vector::new(displacement[0], displacement[1]);
        let destination = at.translation + wanted;
        finite2("slide_destination", [destination.x, destination.y])?;
        crate::validate::finite("slide_distance", wanted.length())?;
        let mut result = SlideMotion2d {
            translation: [0.0; 2],
            remaining: displacement,
            collisions: Vec::new(),
            started_penetrating: false,
            iteration_limit_reached: false,
        };
        if let Some(hit) = self.penetration(probe.as_ref(), at, filter, &mut include) {
            result.started_penetrating = true;
            result.collisions.push(hit);
            return Ok(result);
        }
        let start = at.translation;
        let mut remaining = wanted;
        for _ in 0..options.max_iterations {
            let distance = remaining.length();
            if distance <= f32::EPSILON {
                remaining = r2::Vector::ZERO;
                break;
            }
            let direction = remaining / distance;
            let cast_options = ShapeCastOptions {
                max_time_of_impact: distance,
                target_distance: options.skin,
                stop_at_penetration: false,
                compute_impact_geometry_on_penetration: true,
            };
            let Some(hit) = self.slide_cast(
                probe.as_ref(),
                at,
                direction,
                cast_options,
                filter,
                &mut include,
            ) else {
                at.translation += remaining;
                remaining = r2::Vector::ZERO;
                break;
            };
            at.translation += direction * hit.distance;
            remaining -= direction * hit.distance;
            let normal = r2::Vector::new(hit.normal[0], hit.normal[1]);
            remaining = if let Some(slope) = slope {
                slope.project(remaining, normal)
            } else {
                remaining - normal * remaining.dot(normal).min(0.0)
            };
            result.collisions.push(hit);
        }
        let translation = at.translation - start;
        finite2("slide_translation", [translation.x, translation.y])?;
        finite2("slide_remaining", [remaining.x, remaining.y])?;
        result.translation = [translation.x, translation.y];
        result.remaining = [remaining.x, remaining.y];
        result.iteration_limit_reached = remaining.length() > f32::EPSILON;
        Ok(result)
    }

    pub(super) fn penetration(
        &self,
        probe: &dyn Shape,
        at: r2::Pose,
        filter: RaycastFilter2d,
        include: &mut impl FnMut(EntityId) -> bool,
    ) -> Option<ShapeHit2d> {
        let mut first = None;
        self.each_piece(filter, include, |entity, piece, pose| {
            if first.is_none()
                && query::contact(&at, probe, &pose, piece, 0.0)
                    .is_ok_and(|contact| contact.is_some_and(|contact| contact.dist < 0.0))
            {
                first = Some(ShapeHit2d {
                    entity,
                    point: [at.translation.x, at.translation.y],
                    normal: [0.0; 2],
                    distance: 0.0,
                });
            }
        });
        first
    }

    pub(super) fn slide_cast(
        &self,
        probe: &dyn Shape,
        at: r2::Pose,
        direction: r2::Vector,
        options: ShapeCastOptions,
        filter: RaycastFilter2d,
        include: &mut impl FnMut(EntityId) -> bool,
    ) -> Option<ShapeHit2d> {
        let mut closest: Option<ShapeHit2d> = None;
        self.each_piece(filter, include, |entity, piece, pose| {
            let Ok(Some(hit)) = query::cast_shapes(
                &at,
                direction,
                probe,
                &pose,
                r2::Vector::ZERO,
                piece,
                options,
            ) else {
                return;
            };
            let normal = pose.rotation.transform_vector(hit.normal2);
            // Tangential/separating contact must not repeatedly consume the
            // budget at zero distance while hiding an obstacle farther ahead.
            if direction.dot(normal) >= -f32::EPSILON
                || closest.as_ref().is_some_and(|old| {
                    hit.time_of_impact
                        .total_cmp(&old.distance)
                        .then(entity.cmp(&old.entity))
                        .is_ge()
                })
            {
                return;
            }
            let point = pose.transform_point(hit.witness2);
            closest = Some(ShapeHit2d {
                entity,
                point: [point.x, point.y],
                normal: [normal.x, normal.y],
                distance: hit.time_of_impact,
            });
        });
        closest
    }
}
