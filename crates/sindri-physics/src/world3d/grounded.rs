//! Read-only carry followed by Rapier movement and classified final support.
use sindri_core::EntityId;

use super::{
    PhysicsWorld3d, build, r3,
    validation::{finite3, validate_pose},
};
use crate::{
    ColliderShape3d, GroundOptions3d, GroundedCharacterMotion3d, GroundedCharacterOptions3d,
    PhysicsError, PhysicsPose3d, PlatformCarry3d, RaycastFilter3d,
};

impl PhysicsWorld3d {
    /// Proposes platform carry, movement and classified endpoint support.
    /// The host applies total translation once and advances support snapshots.
    /// Carry sweeps a straight chord with fixed probe orientation; no impulses,
    /// gameplay gravity or jump velocity inheritance are introduced.
    ///
    /// # Errors
    /// Rejects invalid movement/support poses, overflowing bounds/displacements
    /// and nonfinite query geometry before returning a proposal.
    pub fn move_character_grounded(
        &self,
        shape: ColliderShape3d,
        pose: PhysicsPose3d,
        displacement: [f32; 3],
        options: GroundedCharacterOptions3d,
        filter: RaycastFilter3d,
    ) -> Result<GroundedCharacterMotion3d, PhysicsError> {
        self.move_character_grounded_where(shape, pose, displacement, options, filter, |_| true)
    }

    /// [`Self::move_character_grounded`] with a stable whole-entity predicate.
    /// The predicate may be called repeatedly across support/carry/movement.
    ///
    /// # Errors
    /// Has the validation contract of [`Self::move_character_grounded`].
    pub fn move_character_grounded_where(
        &self,
        shape: ColliderShape3d,
        pose: PhysicsPose3d,
        displacement: [f32; 3],
        options: GroundedCharacterOptions3d,
        filter: RaycastFilter3d,
        include: impl Fn(EntityId) -> bool,
    ) -> Result<GroundedCharacterMotion3d, PhysicsError> {
        // Validate even if support is missing or filtered, before predicates run.
        super::character::checked_request(shape, pose, displacement, options.movement)?;
        if let Some(support) = options.platform_support {
            validate_pose(support.previous_pose)?;
        }
        self.spatial.character_tree()?;
        let platform = self.carry(shape, pose, options, filter, &include)?;
        let carry = platform.as_ref().map_or(r3::Vector::ZERO, |platform| {
            r3::Vector::from_array(platform.motion.translation)
        });
        let carried = translated(pose, carry)?;
        let movement = self.move_character_where(
            shape,
            carried,
            displacement,
            options.movement,
            filter,
            &include,
        )?;
        let translation = carry + r3::Vector::from_array(movement.translation);
        finite3("character_total_translation", translation.to_array())?;
        let destination = translated(pose, translation)?;
        let ground = self.probe_ground_where(
            shape,
            destination,
            ground_options(options),
            filter,
            &include,
        )?;
        let up = r3::Vector::from_array(options.movement.up).normalize();
        let grounded = ground.walkable && r3::Vector::from_array(displacement).dot(up) <= 0.0;
        Ok(GroundedCharacterMotion3d {
            translation: translation.to_array(),
            movement,
            ground,
            platform,
            grounded,
        })
    }

    fn carry(
        &self,
        shape: ColliderShape3d,
        pose: PhysicsPose3d,
        options: GroundedCharacterOptions3d,
        filter: RaycastFilter3d,
        include: &impl Fn(EntityId) -> bool,
    ) -> Result<Option<PlatformCarry3d>, PhysicsError> {
        let Some(support) = options.platform_support else {
            return Ok(None);
        };
        if filter.exclude == Some(support.entity)
            || !self.contains(support.entity)
            || !include(support.entity)
        {
            return Ok(None);
        }
        let ground =
            self.ground_on_support(shape, pose, ground_options(options), support, filter)?;
        if !ground.walkable {
            return Ok(None);
        }
        let current_pose = self.pose(support.entity)?;
        let previous = build::pose(support.previous_pose);
        let current = build::pose(current_pose);
        let origin = r3::Vector::from_array(pose.position);
        let local = previous.inverse_transform_point(origin);
        let requested = current.transform_point(local) - origin;
        finite3("platform_displacement", requested.to_array())?;
        let movement_options = crate::CharacterOptions3d {
            slide: false,
            snap_distance: 0.0,
            step_height: 0.0,
            ..options.movement
        };
        let motion = if requested
            .to_array()
            .into_iter()
            .any(|axis| axis.abs() > 0.0)
        {
            self.move_character_where(
                shape,
                pose,
                requested.to_array(),
                movement_options,
                filter,
                |entity| entity != support.entity && include(entity),
            )?
        } else {
            // No support travel must not add a second stationary correction.
            crate::CharacterMotion3d {
                translation: [0.0; 3],
                grounded: false,
                sliding_down_slope: false,
                collisions: Vec::new(),
            }
        };
        Ok(Some(PlatformCarry3d {
            entity: support.entity,
            current_pose,
            requested: requested.to_array(),
            motion,
        }))
    }
}

fn translated(pose: PhysicsPose3d, translation: r3::Vector) -> Result<PhysicsPose3d, PhysicsError> {
    let position = (r3::Vector::from_array(pose.position) + translation).to_array();
    finite3("character_carried_destination", position)?;
    Ok(PhysicsPose3d { position, ..pose })
}

fn ground_options(options: GroundedCharacterOptions3d) -> GroundOptions3d {
    GroundOptions3d {
        up: options.movement.up,
        max_slope_angle: options.movement.max_slope_angle,
        max_distance: 0.0,
        skin: options.movement.skin,
    }
}
