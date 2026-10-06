//! Compose movement and support without storing gameplay state in physics.

use sindri_core::EntityId;

use super::{PhysicsWorld2d, r2};
use crate::validate::finite2;
use crate::{
    ColliderShape2d, GroundedSlideMotion2d, GroundedSlideOptions2d, PhysicsError, PhysicsPose2d,
    RaycastFilter2d,
};

impl PhysicsWorld2d {
    /// Computes sliding, then support and an optional downward snap.
    ///
    /// Snapping defaults off. Requested upward motion suppresses snapping and
    /// grounded state, even if a ceiling blocks ascent. Steep support never
    /// snaps. Does not mutate a body, store prior grounded state, depenetrate,
    /// limit slope motion, step, carry platforms or apply one-way policy.
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
        let slide = self.move_and_slide_where(
            shape,
            pose,
            displacement,
            options.slide,
            filter,
            &mut include,
        )?;
        let endpoint = PhysicsPose2d {
            position: [
                pose.position[0] + slide.translation[0],
                pose.position[1] + slide.translation[1],
            ],
            rotation: pose.rotation,
        };
        let up = r2::Vector::new(options.up[0], options.up[1]).normalize();
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
        Ok(GroundedSlideMotion2d {
            translation,
            slide,
            ground,
            snap_translation: [snap.x, snap.y],
            grounded,
        })
    }
}
