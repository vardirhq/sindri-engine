//! Support probing shares the movement sweep's impact geometry and filtering.

use rapier2d::parry::query::{self, ShapeCastOptions};
use rapier2d::parry::shape::Shape;
use sindri_core::EntityId;

use super::slope::SlopeLimit2d;
use super::sweep::{query_pose, query_shape};
use super::{PhysicsWorld2d, r2};
use crate::validate::finite2;
use crate::{
    ColliderShape2d, GroundOptions2d, GroundProbe2d, PhysicsError, PhysicsPose2d, RaycastFilter2d,
    ShapeHit2d,
};

impl PhysicsWorld2d {
    /// Finds the nearest surface along negative up and classifies its slope.
    ///
    /// Returns steep surfaces as unwalkable instead of probing through them.
    /// Initial penetration blocks classification. A hit reports travel to the
    /// skin; zero travel works for touching/inside-skin support. Does not snap,
    /// mutate a body or implement jump/platform/one-way movement policy.
    ///
    /// # Errors
    /// Rejects invalid shape, pose, options or overflowing probe destination.
    pub fn probe_ground(
        &self,
        shape: ColliderShape2d,
        pose: PhysicsPose2d,
        options: GroundOptions2d,
        filter: RaycastFilter2d,
    ) -> Result<GroundProbe2d, PhysicsError> {
        self.probe_ground_where(shape, pose, options, filter, |_| true)
    }

    /// [`Self::probe_ground`] with an additional whole-entity predicate.
    ///
    /// The predicate must be stable during the call, as for swept movement.
    ///
    /// # Errors
    /// As [`Self::probe_ground`].
    pub fn probe_ground_where(
        &self,
        shape: ColliderShape2d,
        pose: PhysicsPose2d,
        options: GroundOptions2d,
        filter: RaycastFilter2d,
        mut include: impl FnMut(EntityId) -> bool,
    ) -> Result<GroundProbe2d, PhysicsError> {
        let shape = query_shape(shape)?;
        let at = query_pose(pose)?;
        options.validate()?;
        let up = r2::Vector::new(options.up[0], options.up[1]).normalize();
        let destination = at.translation - up * options.max_distance;
        finite2("ground_destination", [destination.x, destination.y])?;
        if let Some(hit) = self.penetration(shape.as_ref(), at, filter, &mut include) {
            return Ok(GroundProbe2d {
                hit: Some(hit),
                walkable: false,
                started_penetrating: true,
            });
        }
        let hit = self
            .skin_support(shape.as_ref(), at, up, options.skin, filter, &mut include)
            .or_else(|| {
                self.slide_cast(
                    shape.as_ref(),
                    at,
                    -up,
                    ShapeCastOptions {
                        max_time_of_impact: options.max_distance,
                        target_distance: options.skin,
                        stop_at_penetration: false,
                        compute_impact_geometry_on_penetration: true,
                    },
                    filter,
                    &mut include,
                )
            });
        let slope = SlopeLimit2d::new(up, options.max_slope_angle);
        let walkable =
            hit.is_some_and(|hit| slope.walkable(r2::Vector::new(hit.normal[0], hit.normal[1])));
        Ok(GroundProbe2d {
            hit,
            walkable,
            started_penetrating: false,
        })
    }

    fn skin_support(
        &self,
        probe: &dyn Shape,
        at: r2::Pose,
        up: r2::Vector,
        skin: f32,
        filter: RaycastFilter2d,
        include: &mut impl FnMut(EntityId) -> bool,
    ) -> Option<ShapeHit2d> {
        let mut first = None;
        // Sweeps can stop slightly early on rotated/curved surfaces. Accept
        // one percent of skin plus one epsilon so a snapped endpoint retains
        // support. Cap prediction to avoid overflowing a valid finite skin.
        let prediction = (skin.mul_add(0.01, skin) + f32::EPSILON).min(f32::MAX);
        self.each_piece(filter, include, |entity, piece, pose| {
            let Ok(Some(contact)) = query::contact(&at, probe, &pose, piece, prediction) else {
                return;
            };
            if first.is_none() && contact.normal2.dot(up) > f32::EPSILON {
                first = Some(ShapeHit2d {
                    entity,
                    point: [contact.point2.x, contact.point2.y],
                    normal: [contact.normal2.x, contact.normal2.y],
                    distance: 0.0,
                });
            }
        });
        first
    }
}
