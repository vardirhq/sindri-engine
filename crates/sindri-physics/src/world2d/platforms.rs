//! Carry a verified previous support point to its current synchronized pose.

use rapier2d::parry::query;
use sindri_core::EntityId;

use super::slide::{SlideRequest2d, contact_prediction};
use super::slope::SlopeLimit2d;
use super::sweep::query_pose;
use super::{PhysicsWorld2d, r2};
use crate::validate::finite2;
use crate::{GroundedSlideOptions2d, PhysicsError, PlatformCarry2d};

impl PhysicsWorld2d {
    pub(super) fn carry_platform(
        &self,
        request: SlideRequest2d,
        options: GroundedSlideOptions2d,
        mut include: impl FnMut(EntityId) -> bool,
    ) -> Result<Option<PlatformCarry2d>, PhysicsError> {
        let Some(support) = options.platform_support else {
            return Ok(None);
        };
        let (probe, at, _) = request.checked()?;
        if !self.bodies.contains_key(&support.entity) {
            return Ok(None);
        }
        let current_pose = self.pose(support.entity)?;
        let current = query_pose(current_pose)?;
        let previous = query_pose(support.previous_pose)?;
        let up = r2::Vector::from_array(options.up).normalize();
        let slope = SlopeLimit2d::new(up, options.max_slope_angle);
        let mut first_normal = None;
        let mut penetrating = false;
        let mut invalid = None;
        // Reconstruct current local collider pieces at the previous body pose.
        // The same masks, sensors, exclusion and host predicate apply here.
        self.each_piece(request.filter, &mut include, |entity, piece, posed| {
            if entity != support.entity {
                return;
            }
            let old_piece = previous * current.inverse() * posed;
            if let Err(error) = finite2("platform_previous_piece", old_piece.translation.to_array())
            {
                invalid = Some(error);
                return;
            }
            let Ok(Some(contact)) = query::contact(
                &at,
                probe.as_ref(),
                &old_piece,
                piece,
                contact_prediction(options.slide.skin),
            ) else {
                return;
            };
            penetrating |= contact.dist < 0.0;
            if first_normal.is_none() && contact.normal2.dot(up) > f32::EPSILON {
                first_normal = Some(contact.normal2);
            }
        });
        if let Some(error) = invalid {
            return Err(error);
        }
        if penetrating || !first_normal.is_some_and(|normal| slope.walkable(normal)) {
            return Ok(None);
        }
        let local_point = previous.inverse_transform_point(at.translation);
        finite2("platform_local_point", local_point.to_array())?;
        let destination = current.transform_point(local_point);
        finite2("platform_destination", destination.to_array())?;
        let displacement = destination - at.translation;
        finite2("platform_displacement", displacement.to_array())?;
        let motion = self.slide_with_policy(
            SlideRequest2d {
                displacement: displacement.to_array(),
                ..request
            },
            None,
            |entity| entity != support.entity && include(entity),
        )?;
        Ok(Some(PlatformCarry2d {
            entity: support.entity,
            current_pose,
            requested_translation: displacement.to_array(),
            motion,
        }))
    }
}
