//! Solve platforms first, then apply collision-limited displacement once.
use glam::{Quat, Vec3};
use sindri_core::{EntityId, World};
use sindri_physics::{
    GroundOptions3d, GroundedCharacterOptions3d, PhysicsError, PhysicsPose3d, PhysicsWorld3d,
    PlatformSupport3d, RaycastFilter3d,
};

use super::{Character, SceneCharacters3d, requests::MotionRequest};
use crate::PhysicsSyncError;

impl SceneCharacters3d {
    /// Hold characters stationary and capture support before platforms advance.
    pub fn hold(
        &mut self,
        world: &World,
        physics: &mut PhysicsWorld3d,
    ) -> Result<(), PhysicsSyncError> {
        self.supports.clear();
        for (&entity, &character) in &self.authored {
            physics.set_linear_velocity(entity, [0.0; 3])?;
            physics.set_angular_velocity(entity, [0.0; 3])?;
            if !character.settings.carry_platforms
                || self
                    .states
                    .get(&entity)
                    .is_some_and(|motion| !motion.grounded)
            {
                continue;
            }
            let movement = character.settings.movement;
            let ground = physics.probe_ground_where(
                character.probe.shape,
                probe_pose(physics.pose(entity)?, character),
                GroundOptions3d {
                    up: movement.up,
                    max_slope_angle: movement.max_slope_angle,
                    skin: movement.skin,
                    max_distance: 0.0,
                },
                filter(entity, character),
                |other| world.is_active(other) && !self.authored.contains_key(&other),
            )?;
            if ground.walkable
                && let Some(hit) = ground.hit
            {
                self.supports.insert(
                    entity,
                    PlatformSupport3d {
                        entity: hit.entity,
                        previous_pose: physics.pose(hit.entity)?,
                    },
                );
            }
        }
        Ok(())
    }

    pub fn apply(
        &mut self,
        world: &World,
        physics: &mut PhysicsWorld3d,
    ) -> Result<(), PhysicsSyncError> {
        // Consume each successful request once, even if a later backend query fails.
        for (&entity, &character) in &self.authored {
            let request = self
                .requests
                .motion
                .get(&entity)
                .copied()
                .unwrap_or(MotionRequest {
                    displacement: [0.0; 3],
                    snap: false,
                });
            let pose = physics.pose(entity)?;
            let mut options = character.settings.movement;
            if !request.snap {
                options.snap_distance = 0.0;
            }
            let motion = physics.move_character_grounded_where(
                character.probe.shape,
                probe_pose(pose, character),
                request.displacement,
                GroundedCharacterOptions3d {
                    movement: options,
                    platform_support: self.supports.get(&entity).copied(),
                },
                filter(entity, character),
                |other| world.is_active(other),
            )?;
            let position =
                (Vec3::from_array(pose.position) + Vec3::from_array(motion.translation)).to_array();
            if !position.into_iter().all(f32::is_finite) {
                return Err(PhysicsError::NonFinite("character_result_destination").into());
            }
            if motion.translation.into_iter().any(|axis| axis.abs() > 0.0) {
                physics.move_to(entity, PhysicsPose3d { position, ..pose })?;
            }
            self.states.insert(entity, motion);
            self.requests.motion.remove(&entity);
            self.supports.remove(&entity);
        }
        Ok(())
    }
}

pub(super) fn probe_pose(body: PhysicsPose3d, character: Character) -> PhysicsPose3d {
    let rotation = Quat::from_array(body.rotation).normalize();
    PhysicsPose3d {
        position: (Vec3::from_array(body.position)
            + rotation * Vec3::from_array(character.probe.offset))
        .to_array(),
        rotation: (rotation * Quat::from_array(character.probe.rotation).normalize()).to_array(),
    }
}

pub(super) fn filter(entity: EntityId, character: Character) -> RaycastFilter3d {
    RaycastFilter3d {
        mask: character.probe.layers.filter,
        include_sensors: false,
        exclude: Some(entity),
    }
}
