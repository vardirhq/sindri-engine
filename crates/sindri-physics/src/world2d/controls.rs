//! Live body controls, kept behind the Sindri entity boundary.

use rapier2d::prelude as r2;
use sindri_core::EntityId;

use super::PhysicsWorld2d;
use crate::validate::{finite2, validate_pose2d};
use crate::{PhysicsError, PhysicsPose2d, RigidBodyKind};

impl PhysicsWorld2d {
    /// Whether swept collision is enabled on this body.
    pub fn continuous_collision(&self, entity: EntityId) -> Result<bool, PhysicsError> {
        Ok(self.backend.bodies[self.record(entity)?.body].is_ccd_enabled())
    }

    /// Toggles swept collision without replacing the body or its joints.
    /// Only dynamic bodies can opt into this control; sensors remain discrete.
    pub fn set_continuous_collision(
        &mut self,
        entity: EntityId,
        enabled: bool,
    ) -> Result<(), PhysicsError> {
        let record = self.record(entity)?.clone();
        if record.kind != RigidBodyKind::Dynamic {
            return Err(PhysicsError::WrongBodyKind(
                entity,
                "set continuous collision",
                record.kind,
            ));
        }
        self.backend.bodies[record.body].enable_ccd(enabled);
        Ok(())
    }

    pub fn linear_velocity(&self, entity: EntityId) -> Result<[f32; 2], PhysicsError> {
        if let Some(record) = self.bodies.get(&entity) {
            let velocity = self.backend.bodies[record.body].linvel();
            return Ok([velocity.x, velocity.y]);
        }
        if let Some(velocity) = self.pending_velocity.get(&entity) {
            return Ok(*velocity);
        }
        Err(PhysicsError::MissingEntity(entity))
    }

    pub fn set_linear_velocity(
        &mut self,
        entity: EntityId,
        velocity: [f32; 2],
    ) -> Result<(), PhysicsError> {
        finite2("linear_velocity", velocity)?;
        let record = self.record(entity)?.clone();
        if !matches!(
            record.kind,
            RigidBodyKind::Dynamic | RigidBodyKind::KinematicVelocity
        ) {
            return Err(PhysicsError::WrongBodyKind(
                entity,
                "set linear velocity",
                record.kind,
            ));
        }
        self.backend.bodies[record.body]
            .set_linvel(r2::Vector::new(velocity[0], velocity[1]), true);
        Ok(())
    }

    pub fn apply_impulse(
        &mut self,
        entity: EntityId,
        impulse: [f32; 2],
    ) -> Result<(), PhysicsError> {
        finite2("impulse", impulse)?;
        let record = self.record(entity)?.clone();
        if record.kind != RigidBodyKind::Dynamic {
            return Err(PhysicsError::WrongBodyKind(
                entity,
                "apply impulse",
                record.kind,
            ));
        }
        self.backend.bodies[record.body]
            .apply_impulse(r2::Vector::new(impulse[0], impulse[1]), true);
        Ok(())
    }

    pub fn set_kinematic_target(
        &mut self,
        entity: EntityId,
        pose: PhysicsPose2d,
    ) -> Result<(), PhysicsError> {
        validate_pose2d(pose)?;
        let record = self.record(entity)?.clone();
        if record.kind != RigidBodyKind::KinematicPosition {
            return Err(PhysicsError::WrongBodyKind(
                entity,
                "set kinematic target",
                record.kind,
            ));
        }
        self.backend.bodies[record.body].set_next_kinematic_position(r2::Pose::new(
            r2::Vector::new(pose.position[0], pose.position[1]),
            pose.rotation,
        ));
        Ok(())
    }

    /// Puts a body somewhere else, as a teleport rather than a movement: it
    /// does not sweep through what lies between, and its velocity is kept.
    ///
    /// For a position-kinematic body the move is its next target instead, so
    /// a platform moved this way carries what stands on it.
    pub fn move_to(&mut self, entity: EntityId, pose: PhysicsPose2d) -> Result<(), PhysicsError> {
        validate_pose2d(pose)?;
        let record = self.record(entity)?.clone();
        let target = r2::Pose::new(
            r2::Vector::new(pose.position[0], pose.position[1]),
            pose.rotation,
        );
        let body = &mut self.backend.bodies[record.body];
        if record.kind == RigidBodyKind::KinematicPosition {
            body.set_next_kinematic_position(target);
        } else {
            body.set_position(target, true);
            self.invalidate_contacts(entity);
            self.index_body(entity);
        }
        Ok(())
    }
}
