//! Optional lift/forward/landing proposals with conservative clearance checks.

use sindri_core::EntityId;

use super::slide::SlideRequest2d;
use super::slope::SlopeLimit2d;
use super::{PhysicsWorld2d, r2};
use crate::validate::finite2;
use crate::{GroundedSlideMotion2d, GroundedSlideOptions2d, PhysicsError, PhysicsPose2d};

impl PhysicsWorld2d {
    pub(super) fn try_step(
        &self,
        request: SlideRequest2d,
        options: GroundedSlideOptions2d,
        baseline: &GroundedSlideMotion2d,
        mut include: impl FnMut(EntityId) -> bool,
    ) -> Result<Option<GroundedSlideMotion2d>, PhysicsError> {
        let up = r2::Vector::new(options.up[0], options.up[1]).normalize();
        let Some(horizontal) = step_direction(request, options, baseline, up) else {
            return Ok(None);
        };
        let slope = SlopeLimit2d::new(up, options.max_slope_angle);
        let mut support_options = options.ground_options();
        support_options.max_distance = 0.0;
        if !self
            .probe_ground_where(
                request.shape,
                request.pose,
                support_options,
                request.filter,
                &mut include,
            )?
            .walkable
        {
            return Ok(None);
        }
        let lift = up * options.step_height;
        let clearance = self.slide_with_policy(
            SlideRequest2d {
                displacement: lift.to_array(),
                ..request
            },
            None,
            &mut include,
        )?;
        // A ceiling/overhang cannot redirect lift sideways into a step. Require
        // the full lift to be clear, preserving the configured surface skin.
        if !clearance.collisions.is_empty()
            || clearance.started_penetrating
            || clearance.iteration_limit_reached
        {
            return Ok(None);
        }
        let lifted = translated_pose(request.pose, lift)?;
        let forward = self.slide_with_policy(
            SlideRequest2d {
                pose: lifted,
                displacement: horizontal.to_array(),
                ..request
            },
            Some(slope),
            &mut include,
        )?;
        let direction = horizontal.normalize();
        let progress = r2::Vector::from_array(forward.translation).dot(direction);
        let old_progress = r2::Vector::from_array(baseline.translation).dot(direction);
        if forward.started_penetrating
            || forward.iteration_limit_reached
            || progress <= old_progress + options.slide.skin
        {
            return Ok(None);
        }
        let endpoint = translated_pose(lifted, r2::Vector::from_array(forward.translation))?;
        support_options.max_distance = options.step_height;
        let ground = self.probe_ground_where(
            request.shape,
            endpoint,
            support_options,
            request.filter,
            &mut include,
        )?;
        if !ground.walkable {
            return Ok(None);
        }
        let landing = -up * ground.hit.map_or(0.0, |hit| hit.distance);
        let translation = lift + r2::Vector::from_array(forward.translation) + landing;
        let final_pose = translated_pose(request.pose, translation)?;
        support_options.max_distance = 0.0;
        // Confirm the proposed endpoint still has walkable support. This also
        // rejects numerical overlap or an incline climbed beyond the step cap.
        if translation.dot(up) > options.step_height + f32::EPSILON
            || !self
                .probe_ground_where(
                    request.shape,
                    final_pose,
                    support_options,
                    request.filter,
                    &mut include,
                )?
                .walkable
        {
            return Ok(None);
        }
        Ok(Some(GroundedSlideMotion2d {
            translation: translation.to_array(),
            slide: forward,
            ground,
            snap_translation: landing.to_array(),
            step_translation: lift.to_array(),
            platform: None,
            grounded: true,
        }))
    }
}

fn translated_pose(
    pose: PhysicsPose2d,
    translation: r2::Vector,
) -> Result<PhysicsPose2d, PhysicsError> {
    let position = [
        pose.position[0] + translation.x,
        pose.position[1] + translation.y,
    ];
    finite2("step_destination", position)?;
    Ok(PhysicsPose2d {
        position,
        rotation: pose.rotation,
    })
}

fn step_direction(
    request: SlideRequest2d,
    options: GroundedSlideOptions2d,
    baseline: &GroundedSlideMotion2d,
    up: r2::Vector,
) -> Option<r2::Vector> {
    let wanted = r2::Vector::from_array(request.displacement);
    let horizontal = wanted - up * wanted.dot(up);
    let slope = SlopeLimit2d::new(up, options.max_slope_angle);
    if options.step_height <= 0.0
        || baseline.slide.started_penetrating
        || wanted.dot(up) > 0.0
        || horizontal.length() <= options.slide.skin
        || !baseline.slide.collisions.iter().any(|hit| {
            let normal = r2::Vector::from_array(hit.normal);
            normal.dot(horizontal) < -f32::EPSILON && !slope.walkable(normal)
        })
    {
        None
    } else {
        Some(horizontal)
    }
}
