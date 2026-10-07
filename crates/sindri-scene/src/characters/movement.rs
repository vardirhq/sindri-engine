//! Resolve movement against the current solve, then update collider poses once.
use super::requests::MotionRequest;
use super::{Character, CharacterState, SceneCharacters2d};
use crate::PhysicsSyncError;
use sindri_core::{EntityId, World};
use sindri_physics::{PhysicsPose2d, PhysicsWorld2d, PlatformSupport2d, RaycastFilter2d};
use std::time::Duration;

impl SceneCharacters2d {
    /// Freeze solver velocity and seed fresh support before platforms advance.
    pub fn hold(
        &mut self,
        world: &World,
        physics: &mut PhysicsWorld2d,
    ) -> Result<(), PhysicsSyncError> {
        for (&entity, &character) in &self.authored {
            physics.set_linear_velocity(entity, [0.0; 2])?;
            physics.set_angular_velocity(entity, 0.0)?;
            let pose = physics.pose(entity)?;
            let state = self.states.entry(entity).or_default();
            if state.last_pose.is_some_and(|old| {
                (old.position[0] - pose.position[0]).abs() > 1.0e-4
                    || (old.position[1] - pose.position[1]).abs() > 1.0e-4
                    || (old.rotation - pose.rotation).abs() > 1.0e-4
            }) {
                *state = CharacterState::default();
            }
            if character.settings.carry_platforms
                && state.motion.is_none()
                && state.support.is_none()
            {
                let mut options = character.settings.options();
                options.snap_distance = 0.0;
                options.step_height = 0.0;
                let motion = physics.move_and_slide_grounded_where(
                    character.probe.shape,
                    probe_pose(pose, character),
                    [0.0; 2],
                    options,
                    filter(entity, character),
                    |other| world.is_active(other),
                )?;
                state.support = support_of(physics, &motion);
            }
        }
        Ok(())
    }

    pub fn apply(
        &mut self,
        world: &World,
        physics: &mut PhysicsWorld2d,
        delta: Duration,
    ) -> Result<(), PhysicsSyncError> {
        let requests = std::mem::take(&mut self.requests.motion);
        let drops = std::mem::take(&mut self.requests.drop);
        for (&entity, &character) in &self.authored {
            let state = self.states.entry(entity).or_default();
            if let Some(seconds) = drops.get(&entity) {
                state.drop_seconds = *seconds;
            }
            let request = requests.get(&entity).copied().unwrap_or(MotionRequest {
                displacement: [0.0; 2],
                snap: state.motion.as_ref().is_some_and(|motion| motion.grounded),
            });
            apply_one(world, physics, entity, character, state, request)?;
            state.drop_seconds = (state.drop_seconds - delta.as_secs_f32()).max(0.0);
        }
        Ok(())
    }
}

fn apply_one(
    world: &World,
    physics: &mut PhysicsWorld2d,
    entity: EntityId,
    character: Character,
    state: &mut CharacterState,
    request: MotionRequest,
) -> Result<(), PhysicsSyncError> {
    let body_pose = physics.pose(entity)?;
    let mut options = character.settings.options();
    options.platform_support = character
        .settings
        .carry_platforms
        .then_some(state.support)
        .flatten();
    options.drop_through = state.drop_seconds > 0.0;
    if !request.snap {
        options.snap_distance = 0.0;
    }
    let motion = physics.move_and_slide_grounded_where(
        character.probe.shape,
        probe_pose(body_pose, character),
        request.displacement,
        options,
        filter(entity, character),
        |other| world.is_active(other),
    )?;
    let pose = PhysicsPose2d {
        position: [
            body_pose.position[0] + motion.translation[0],
            body_pose.position[1] + motion.translation[1],
        ],
        rotation: body_pose.rotation,
    };
    if motion.translation.iter().any(|axis| axis.abs() > 0.0) {
        physics.move_to(entity, pose)?;
    }
    state.support = if character.settings.carry_platforms {
        support_of(physics, &motion)
    } else {
        None
    };
    state.last_pose = Some(pose);
    state.motion = Some(motion);
    Ok(())
}

fn probe_pose(body_pose: PhysicsPose2d, character: Character) -> PhysicsPose2d {
    let (sin, cos) = body_pose.rotation.sin_cos();
    let offset = character.probe.offset;
    PhysicsPose2d {
        position: [
            body_pose.position[0] + cos * offset[0] - sin * offset[1],
            body_pose.position[1] + sin * offset[0] + cos * offset[1],
        ],
        rotation: body_pose.rotation + character.probe.rotation,
    }
}

fn filter(entity: EntityId, character: Character) -> RaycastFilter2d {
    RaycastFilter2d {
        mask: character.probe.layers.filter,
        include_sensors: false,
        exclude: Some(entity),
    }
}

fn support_of(
    physics: &PhysicsWorld2d,
    motion: &sindri_physics::GroundedSlideMotion2d,
) -> Option<PlatformSupport2d> {
    if !motion.grounded {
        return None;
    }
    let entity = motion.ground.hit?.entity;
    physics
        .pose(entity)
        .ok()
        .map(|previous_pose| PlatformSupport2d {
            entity,
            previous_pose,
        })
}
