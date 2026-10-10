//! Rapier character movement over the engine's current-pose query index.

use rapier3d::control::{CharacterAutostep, CharacterLength, KinematicCharacterController};
use rapier3d::parry::bounding_volume::BoundingVolume;
use sindri_core::EntityId;

use super::sweep::query_shape;
use super::validation::{finite3, validate_pose};
use super::{PhysicsWorld3d, build, r3};
use crate::validate::finite;
use crate::{
    CharacterCollision3d, CharacterMotion3d, CharacterOptions3d, ColliderShape3d, PhysicsError,
    PhysicsPose3d, RaycastFilter3d, ShapeHit3d,
};

impl PhysicsWorld3d {
    /// Computes collision-limited 3D movement without changing the world.
    ///
    /// Uses Rapier's fixed-orientation character controller, including slope
    /// limits, optional steps/snap and bounded stationary depenetration. Reads
    /// current poses after insertion/teleport, even before a solver step.
    /// Platform carry and physical push impulses are separate future policies.
    /// Exact collision ties and iteration order remain backend-owned.
    ///
    /// # Errors
    /// Rejects invalid shape, pose, options, displacement/endpoints, overflowing
    /// probe/obstacle bounds and non-finite backend results.
    pub fn move_character(
        &self,
        shape: ColliderShape3d,
        pose: PhysicsPose3d,
        displacement: [f32; 3],
        options: CharacterOptions3d,
        filter: RaycastFilter3d,
    ) -> Result<CharacterMotion3d, PhysicsError> {
        self.move_character_where(shape, pose, displacement, options, filter, |_| true)
    }

    /// [`Self::move_character`] with a stable whole-entity predicate.
    /// The predicate can be called repeatedly across movement/support probes.
    ///
    /// # Errors
    /// Has the validation contract of [`Self::move_character`].
    pub fn move_character_where(
        &self,
        shape: ColliderShape3d,
        pose: PhysicsPose3d,
        displacement: [f32; 3],
        options: CharacterOptions3d,
        filter: RaycastFilter3d,
        include: impl Fn(EntityId) -> bool,
    ) -> Result<CharacterMotion3d, PhysicsError> {
        let (shape, at, wanted) = checked_request(shape, pose, displacement, options)?;
        let accept = |handle, collider: &r3::Collider| {
            self.collider_entities.get(&handle).is_some_and(|&entity| {
                collider.is_enabled()
                    && filter.exclude != Some(entity)
                    && (filter.include_sensors || !collider.is_sensor())
                    && collider.collision_groups().memberships.bits() & filter.mask != 0
                    && include(entity)
            })
        };
        let queries = r3::QueryPipeline {
            dispatcher: self.backend.narrow_phase.query_dispatcher(),
            bvh: self.spatial.character_tree()?,
            bodies: &self.backend.bodies,
            colliders: &self.backend.colliders,
            filter: r3::QueryFilter {
                predicate: Some(&accept),
                ..r3::QueryFilter::default()
            },
        };
        let mut collisions = Vec::new();
        // A displacement query has no simulation timestep. Zero disables
        // Rapier's velocity-based kinematic friction/carry; scene integration
        // must add solved platform motion exactly once under an explicit policy.
        let movement = controller(options).move_shape(
            0.0,
            &queries,
            shape.as_ref(),
            &at,
            wanted,
            |collision| {
                if let Some(&entity) = self.collider_entities.get(&collision.handle) {
                    collisions.push(CharacterCollision3d {
                        hit: ShapeHit3d {
                            entity,
                            point: collision.hit.witness1.to_array(),
                            normal: collision.hit.normal1.to_array(),
                            distance: collision.hit.time_of_impact,
                        },
                        translation_applied: collision.translation_applied.to_array(),
                        translation_remaining: collision.translation_remaining.to_array(),
                    });
                }
            },
        );
        let result = CharacterMotion3d {
            translation: movement.translation.to_array(),
            grounded: movement.grounded,
            sliding_down_slope: movement.is_sliding_down_slope,
            collisions,
        };
        check_result(&result, pose.position)?;
        Ok(result)
    }
}

fn controller(options: CharacterOptions3d) -> KinematicCharacterController {
    KinematicCharacterController {
        up: r3::Vector::from_array(options.up).normalize(),
        offset: CharacterLength::Absolute(options.skin),
        slide: options.slide,
        autostep: (options.step_height > 0.0).then_some(CharacterAutostep {
            max_height: CharacterLength::Absolute(options.step_height),
            min_width: CharacterLength::Absolute(options.step_min_width),
            include_dynamic_bodies: options.step_dynamic_bodies,
        }),
        max_slope_climb_angle: options.max_slope_angle,
        min_slope_slide_angle: options.min_slide_angle,
        snap_to_ground: (options.snap_distance > 0.0)
            .then_some(CharacterLength::Absolute(options.snap_distance)),
        normal_nudge_factor: options.skin * 0.01,
    }
}

fn checked_request(
    shape: ColliderShape3d,
    pose: PhysicsPose3d,
    displacement: [f32; 3],
    options: CharacterOptions3d,
) -> Result<(r3::SharedShape, r3::Pose, r3::Vector), PhysicsError> {
    let shape = query_shape(shape)?;
    validate_pose(pose)?;
    options.validate()?;
    finite3("character_displacement", displacement)?;
    let wanted = r3::Vector::from_array(displacement);
    finite("character_distance", wanted.length())?;
    let at = build::pose(pose);
    finite3(
        "character_destination",
        (at.translation + wanted).to_array(),
    )?;
    let margin =
        options.skin + options.snap_distance + options.step_height + options.step_min_width + 0.05;
    finite("character_query_margin", margin)?;
    finite(
        "character_probe_extent",
        shape.compute_local_aabb().extents().length(),
    )?;
    for point in [at.translation, at.translation + wanted] {
        let bounds = shape
            .compute_aabb(&r3::Pose::from_parts(point, at.rotation))
            .loosened(margin);
        finite3("character_probe_bounds", bounds.mins.to_array())?;
        finite3("character_probe_bounds", bounds.maxs.to_array())?;
    }
    Ok((shape, at, wanted))
}

fn check_result(result: &CharacterMotion3d, origin: [f32; 3]) -> Result<(), PhysicsError> {
    finite3("character_result", result.translation)?;
    finite3(
        "character_result_destination",
        std::array::from_fn(|i| origin[i] + result.translation[i]),
    )?;
    for collision in &result.collisions {
        finite3("character_hit_point", collision.hit.point)?;
        finite3("character_hit_normal", collision.hit.normal)?;
        finite("character_hit_distance", collision.hit.distance)?;
        finite3("character_applied", collision.translation_applied)?;
        finite3("character_remaining", collision.translation_remaining)?;
    }
    Ok(())
}
