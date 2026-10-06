//! A read-only geometric primitive; gameplay decides how to apply its result.

use rapier2d::parry::query::{self, ShapeCastOptions};
use sindri_core::EntityId;

use super::movement_policy::MovementProbe2d;
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
    pub one_way: Option<bool>,
}

impl SlideRequest2d {
    pub(super) fn checked(self) -> Result<(r2::SharedShape, r2::Pose, r2::Vector), PhysicsError> {
        let probe = query_shape(self.shape)?;
        let at = query_pose(self.pose)?;
        self.options.validate()?;
        finite2("slide_displacement", self.displacement)?;
        let wanted = r2::Vector::from_array(self.displacement);
        let destination = at.translation + wanted;
        finite2("slide_destination", destination.to_array())?;
        crate::validate::finite("slide_distance", wanted.length())?;
        Ok((probe, at, wanted))
    }
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
                one_way: None,
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
        let (probe, mut at, wanted) = request.checked()?;
        let SlideRequest2d {
            displacement,
            options,
            filter,
            ..
        } = request;
        let mut result = SlideMotion2d {
            translation: [0.0; 2],
            remaining: displacement,
            collisions: Vec::new(),
            started_penetrating: false,
            iteration_limit_reached: false,
        };
        let movement_probe = |pose| MovementProbe2d {
            shape: probe.as_ref(),
            pose,
            filter,
            one_way: request.one_way,
        };
        if let Some(hit) = self.penetration(movement_probe(at), wanted, &mut include) {
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
            let Some(hit) =
                self.slide_cast(movement_probe(at), direction, cast_options, &mut include)
            else {
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
        probe: MovementProbe2d<'_>,
        direction: r2::Vector,
        include: &mut impl FnMut(EntityId) -> bool,
    ) -> Option<ShapeHit2d> {
        let mut first = None;
        self.each_piece_policy(probe.filter, include, |entity, piece, pose, policy| {
            let Ok(Some(contact)) = query::contact(&probe.pose, probe.shape, &pose, piece, 0.0)
            else {
                return;
            };
            if first.is_none()
                && contact.dist < 0.0
                && probe.allows(piece, pose, policy, direction, contact.normal2)
            {
                first = Some(ShapeHit2d {
                    entity,
                    point: probe.pose.translation.to_array(),
                    normal: [0.0; 2],
                    distance: 0.0,
                });
            }
        });
        first
    }

    pub(super) fn slide_cast(
        &self,
        probe: MovementProbe2d<'_>,
        direction: r2::Vector,
        options: ShapeCastOptions,
        include: &mut impl FnMut(EntityId) -> bool,
    ) -> Option<ShapeHit2d> {
        let mut closest: Option<ShapeHit2d> = None;
        let at = probe.pose;
        self.each_piece_policy(probe.filter, include, |entity, piece, pose, policy| {
            let Ok(Some(hit)) = query::cast_shapes(
                &at,
                direction,
                probe.shape,
                &pose,
                r2::Vector::ZERO,
                piece,
                options,
            ) else {
                return;
            };
            // Near the skin, cast normals can tilt even on a flat box face.
            // Contact geometry at impact is more stable for touching features.
            let mut impact = at;
            impact.translation += direction * hit.time_of_impact;
            let contact = query::contact(
                &impact,
                probe.shape,
                &pose,
                piece,
                contact_prediction(options.target_distance),
            )
            .ok()
            .flatten();
            let normal = contact.as_ref().map_or_else(
                || pose.rotation.transform_vector(hit.normal2),
                |contact| contact.normal2,
            );
            // Tangential/separating contact must not repeatedly consume the
            // budget at zero distance while hiding an obstacle farther ahead.
            if !probe.allows(piece, pose, policy, direction, normal)
                || direction.dot(normal) >= -f32::EPSILON
                || closest.as_ref().is_some_and(|old| {
                    hit.time_of_impact
                        .total_cmp(&old.distance)
                        .then(entity.cmp(&old.entity))
                        .is_ge()
                })
            {
                return;
            }
            let point = contact.as_ref().map_or_else(
                || pose.transform_point(hit.witness2),
                |contact| contact.point2,
            );
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

/// Relative allowance for contact geometry at a sweep's nominal skin.
pub(super) fn contact_prediction(skin: f32) -> f32 {
    (skin.mul_add(0.01, skin) + f32::EPSILON).min(f32::MAX)
}
