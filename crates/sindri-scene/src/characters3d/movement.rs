//! Solve platforms first, then apply collision-limited displacement once.
use glam::{Quat, Vec3};
use sindri_core::World;
use sindri_physics::{PhysicsError, PhysicsPose3d, PhysicsWorld3d, RaycastFilter3d};

use super::{Character, SceneCharacters3d, requests::MotionRequest};
use crate::{PhysicsSyncError, voxel_collision3d::VoxelReach};

impl SceneCharacters3d {
    /// Extend terrain residency for the full requested travel plus geometry policy.
    pub fn reaches(
        &self,
        world: &World,
        plan: &super::CharacterPlan,
    ) -> Result<Vec<VoxelReach>, PhysicsSyncError> {
        plan.iter()
            .map(|(&entity, character)| {
                let pose = crate::physics_sync3d::pose_of(world, entity, None);
                let request = self
                    .requests
                    .motion
                    .get(&entity)
                    .copied()
                    .unwrap_or(MotionRequest {
                        displacement: [0.0; 3],
                        snap: false,
                    });
                let options = character.settings.0;
                let reach = crate::physics_sync3d::extent(&character.probe)
                    + Vec3::from_array(request.displacement).length()
                    + options.skin
                    + options.snap_distance
                    + options.step_height
                    + options.step_min_width
                    + 0.05;
                if !reach.is_finite()
                    || pose
                        .position
                        .iter()
                        .zip(request.displacement)
                        .any(|(axis, travel)| !(axis + travel).is_finite())
                {
                    return Err(PhysicsError::NonFinite("character_destination").into());
                }
                Ok(VoxelReach {
                    position: pose.position,
                    reach,
                })
            })
            .collect()
    }

    /// Prevent solver velocity from becoming a second movement owner.
    pub fn hold(&self, physics: &mut PhysicsWorld3d) -> Result<(), PhysicsSyncError> {
        for &entity in self.authored.keys() {
            physics.set_linear_velocity(entity, [0.0; 3])?;
            physics.set_angular_velocity(entity, [0.0; 3])?;
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
            let mut options = character.settings.0;
            if !request.snap {
                options.snap_distance = 0.0;
            }
            let motion = physics.move_character_where(
                character.probe.shape,
                probe_pose(pose, character),
                request.displacement,
                options,
                RaycastFilter3d {
                    mask: character.probe.layers.filter,
                    include_sensors: false,
                    exclude: Some(entity),
                },
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
        }
        Ok(())
    }
}

fn probe_pose(body: PhysicsPose3d, character: Character) -> PhysicsPose3d {
    let rotation = Quat::from_array(body.rotation).normalize();
    PhysicsPose3d {
        position: (Vec3::from_array(body.position)
            + rotation * Vec3::from_array(character.probe.offset))
        .to_array(),
        rotation: (rotation * Quat::from_array(character.probe.rotation).normalize()).to_array(),
    }
}
