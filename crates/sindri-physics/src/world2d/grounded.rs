//! Compose movement and support without storing gameplay state in physics.

use sindri_core::EntityId;

use super::slide::SlideRequest2d;
use super::slope::SlopeLimit2d;
use super::{PhysicsWorld2d, r2};
use crate::validate::finite2;
use crate::{
    ColliderShape2d, GroundedSlideMotion2d, GroundedSlideOptions2d, PhysicsError, PhysicsPose2d,
    RaycastFilter2d,
};

impl PhysicsWorld2d {
    /// Computes sliding, optional stepping, support and downward snapping.
    ///
    /// Snapping defaults off. Requested upward motion suppresses snapping and
    /// grounded state, even if a ceiling blocks ascent. Steep support never
    /// snaps. Does not mutate a body, store prior grounded state, depenetrate,
    /// carry platforms or apply one-way policy. Steps require starting support
    /// and clear lift/forward/landing sweeps. Steep contacts cannot
    /// create upward motion beyond the remaining request's positive rise;
    /// downhill sliding and explicit jumps remain possible.
    ///
    /// # Errors
    /// Rejects invalid shape, pose, displacement, options or query destination.
    pub fn move_and_slide_grounded(
        &self,
        shape: ColliderShape2d,
        pose: PhysicsPose2d,
        displacement: [f32; 2],
        options: GroundedSlideOptions2d,
        filter: RaycastFilter2d,
    ) -> Result<GroundedSlideMotion2d, PhysicsError> {
        self.move_and_slide_grounded_where(shape, pose, displacement, options, filter, |_| true)
    }

    /// [`Self::move_and_slide_grounded`] with a stable whole-entity predicate.
    ///
    /// The same filter and predicate apply to movement and support probing.
    ///
    /// # Errors
    /// As [`Self::move_and_slide_grounded`].
    pub fn move_and_slide_grounded_where(
        &self,
        shape: ColliderShape2d,
        pose: PhysicsPose2d,
        displacement: [f32; 2],
        options: GroundedSlideOptions2d,
        filter: RaycastFilter2d,
        mut include: impl FnMut(EntityId) -> bool,
    ) -> Result<GroundedSlideMotion2d, PhysicsError> {
        options.validate()?;
        let up = r2::Vector::new(options.up[0], options.up[1]).normalize();
        let slide = self.slide_with_policy(
            SlideRequest2d {
                shape,
                pose,
                displacement,
                options: options.slide,
                filter,
            },
            Some(SlopeLimit2d::new(up, options.max_slope_angle)),
            &mut include,
        )?;
        let endpoint = PhysicsPose2d {
            position: [
                pose.position[0] + slide.translation[0],
                pose.position[1] + slide.translation[1],
            ],
            rotation: pose.rotation,
        };
        let ascending = r2::Vector::new(displacement[0], displacement[1]).dot(up) > 0.0;
        let mut ground_options = options.ground_options();
        if ascending || slide.started_penetrating {
            ground_options.max_distance = 0.0;
        }
        let ground =
            self.probe_ground_where(shape, endpoint, ground_options, filter, &mut include)?;
        let grounded = !ascending && !slide.started_penetrating && ground.walkable;
        let snap = if grounded {
            -up * ground.hit.map_or(0.0, |hit| hit.distance)
        } else {
            r2::Vector::ZERO
        };
        let translation = [slide.translation[0] + snap.x, slide.translation[1] + snap.y];
        finite2("grounded_translation", translation)?;
        finite2(
            "grounded_destination",
            [
                pose.position[0] + translation[0],
                pose.position[1] + translation[1],
            ],
        )?;
        let result = GroundedSlideMotion2d {
            translation,
            slide,
            ground,
            snap_translation: [snap.x, snap.y],
            step_translation: [0.0; 2],
            grounded,
        };
        Ok(self
            .try_step(
                SlideRequest2d {
                    shape,
                    pose,
                    displacement,
                    options: options.slide,
                    filter,
                },
                options,
                &result,
                &mut include,
            )?
            .unwrap_or(result))
    }
}
