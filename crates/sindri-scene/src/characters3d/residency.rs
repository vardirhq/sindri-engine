//! Residency for input before solving and actual support travel before carry.
use glam::{Quat, Vec3};
use sindri_core::{EntityId, World};
use sindri_physics::{PhysicsError, PhysicsWorld3d};

use super::{
    Character, CharacterPlan, SceneCharacters3d, movement::probe_pose, requests::MotionRequest,
};
use crate::{PhysicsSyncError, voxel_collision3d::VoxelReach};

impl SceneCharacters3d {
    pub fn reaches(
        &self,
        world: &World,
        plan: &CharacterPlan,
    ) -> Result<Vec<VoxelReach>, PhysicsSyncError> {
        plan.iter()
            .map(|(&entity, &character)| {
                let pose = crate::physics_sync3d::pose_of(world, entity, None);
                self.reach(entity, character, pose.position, 0.0)
            })
            .collect()
    }

    /// The solver has advanced supports; expand terrain before any rider moves.
    pub fn carry_reaches(
        &self,
        physics: &PhysicsWorld3d,
    ) -> Result<Vec<VoxelReach>, PhysicsSyncError> {
        let mut reaches = Vec::new();
        for (&entity, &support) in &self.supports {
            let character = self.authored[&entity];
            let body = physics.pose(entity)?;
            let probe = probe_pose(body, character);
            let old = support.previous_pose;
            let current = physics.pose(support.entity)?;
            if old == current {
                continue;
            }
            let origin = Vec3::from_array(probe.position);
            let local = Quat::from_array(old.rotation).normalize().conjugate()
                * (origin - Vec3::from_array(old.position));
            let destination = Vec3::from_array(current.position)
                + Quat::from_array(current.rotation).normalize() * local;
            let travel = (destination - origin).length();
            if !travel.is_finite() {
                return Err(PhysicsError::NonFinite("character_carry_reach").into());
            }
            if travel > 0.0 {
                reaches.push(self.reach(entity, character, body.position, travel)?);
            }
        }
        Ok(reaches)
    }

    fn reach(
        &self,
        entity: EntityId,
        character: Character,
        position: [f32; 3],
        carry: f32,
    ) -> Result<VoxelReach, PhysicsSyncError> {
        let request = self
            .requests
            .motion
            .get(&entity)
            .copied()
            .unwrap_or(MotionRequest {
                displacement: [0.0; 3],
                snap: false,
            });
        let options = character.settings.movement;
        let reach = crate::physics_sync3d::extent(&character.probe)
            + Vec3::from_array(request.displacement).length()
            + carry
            + options.skin
            + options.snap_distance
            + options.step_height
            + options.step_min_width
            + 0.05;
        if !reach.is_finite()
            || position
                .iter()
                .zip(request.displacement)
                .any(|(axis, travel)| !(axis + travel).is_finite())
        {
            return Err(PhysicsError::NonFinite("character_destination").into());
        }
        Ok(VoxelReach { position, reach })
    }
}
