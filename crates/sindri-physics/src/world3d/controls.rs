//! Live controls with strict body-kind and finite-input validation.

use super::validation::{finite3, validate_pose};
use super::{PhysicsWorld3d, build, r3};
use crate::{PhysicsError, PhysicsPose3d, RigidBodyKind};
use sindri_core::EntityId;

impl PhysicsWorld3d {
    pub fn linear_velocity(&self, entity: EntityId) -> Result<[f32; 3], PhysicsError> {
        Ok(self.backend.bodies[self.record(entity)?.body]
            .linvel()
            .to_array())
    }

    pub fn angular_velocity(&self, entity: EntityId) -> Result<[f32; 3], PhysicsError> {
        Ok(self.backend.bodies[self.record(entity)?.body]
            .angvel()
            .to_array())
    }

    pub fn set_linear_velocity(
        &mut self,
        entity: EntityId,
        velocity: [f32; 3],
    ) -> Result<(), PhysicsError> {
        finite3("linear_velocity", velocity)?;
        let handle = self.velocity_body(entity)?;
        self.backend.bodies[handle].set_linvel(r3::Vector::from_array(velocity), true);
        Ok(())
    }

    /// Sets radians per second around XYZ. Authored rotation locking keeps zero.
    pub fn set_angular_velocity(
        &mut self,
        entity: EntityId,
        velocity: [f32; 3],
    ) -> Result<(), PhysicsError> {
        finite3("angular_velocity", velocity)?;
        let handle = self.velocity_body(entity)?;
        let body = &mut self.backend.bodies[handle];
        let velocity = if body.is_rotation_locked().into_iter().all(|locked| locked) {
            r3::Vector::ZERO
        } else {
            r3::Vector::from_array(velocity)
        };
        body.set_angvel(velocity, true);
        Ok(())
    }

    pub fn apply_impulse(
        &mut self,
        entity: EntityId,
        impulse: [f32; 3],
    ) -> Result<(), PhysicsError> {
        finite3("impulse", impulse)?;
        let record = self.record(entity)?;
        if record.kind != RigidBodyKind::Dynamic {
            return Err(PhysicsError::WrongBodyKind(
                entity,
                "apply impulse",
                record.kind,
            ));
        }
        let handle = record.body;
        self.backend.bodies[handle].apply_impulse(r3::Vector::from_array(impulse), true);
        Ok(())
    }

    pub fn set_kinematic_target(
        &mut self,
        entity: EntityId,
        pose: PhysicsPose3d,
    ) -> Result<(), PhysicsError> {
        validate_pose(pose)?;
        let record = self.record(entity)?;
        if record.kind != RigidBodyKind::KinematicPosition {
            return Err(PhysicsError::WrongBodyKind(
                entity,
                "set kinematic target",
                record.kind,
            ));
        }
        let handle = record.body;
        self.backend.bodies[handle].set_next_kinematic_position(build::pose(pose));
        Ok(())
    }

    /// Teleports without sweeping, keeping velocity. Position-kinematic bodies
    /// instead retain the pose as their target until the next fixed step.
    pub fn move_to(&mut self, entity: EntityId, pose: PhysicsPose3d) -> Result<(), PhysicsError> {
        validate_pose(pose)?;
        let record = self.record(entity)?;
        let (handle, kind) = (record.body, record.kind);
        let body = &mut self.backend.bodies[handle];
        if kind == RigidBodyKind::KinematicPosition {
            body.set_next_kinematic_position(build::pose(pose));
        } else {
            body.set_position(build::pose(pose), true);
        }
        Ok(())
    }

    fn velocity_body(&self, entity: EntityId) -> Result<r3::RigidBodyHandle, PhysicsError> {
        let record = self.record(entity)?;
        if !matches!(
            record.kind,
            RigidBodyKind::Dynamic | RigidBodyKind::KinematicVelocity
        ) {
            return Err(PhysicsError::WrongBodyKind(
                entity,
                "set velocity",
                record.kind,
            ));
        }
        Ok(record.body)
    }
}
