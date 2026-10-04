//! Forces and rotation, including the spawn-to-synchronization window.

use rapier2d::prelude as r2;
use sindri_core::EntityId;

use super::PhysicsWorld2d;
use crate::validate::{finite, finite2};
use crate::{PhysicsError, RigidBodyKind};

/// A world-space motion request. Forces last one fixed step; impulses act
/// immediately on a live body. Queued requests are replayed in call order.
#[derive(Clone, Copy, Debug)]
pub enum BodyControl2d {
    Force([f32; 2]),
    Torque(f32),
    LinearVelocity([f32; 2]),
    AngularVelocity(f32),
    Impulse([f32; 2]),
    AngularImpulse(f32),
    ImpulseAtPoint { impulse: [f32; 2], point: [f32; 2] },
}

impl BodyControl2d {
    pub(super) fn validate(self, entity: EntityId, kind: RigidBodyKind) -> Result<(), PhysicsError> {
        match self {
            Self::Force(value) => finite2("force", value)?,
            Self::Torque(value) => finite("torque", value)?,
            Self::LinearVelocity(value) => finite2("linear_velocity", value)?,
            Self::AngularVelocity(value) => finite("angular_velocity", value)?,
            Self::Impulse(value) => finite2("impulse", value)?,
            Self::AngularImpulse(value) => finite("angular_impulse", value)?,
            Self::ImpulseAtPoint { impulse, point } => {
                finite2("impulse", impulse)?;
                finite2("impulse_point", point)?;
            }
        }
        let velocity = matches!(self, Self::LinearVelocity(_) | Self::AngularVelocity(_));
        if kind != RigidBodyKind::Dynamic && !(velocity && kind == RigidBodyKind::KinematicVelocity) {
            return Err(PhysicsError::WrongBodyKind(entity, "motion control", kind));
        }
        Ok(())
    }
}

impl PhysicsWorld2d {
    /// Validates the complete request before changing a live body.
    pub fn apply_control(&mut self, entity: EntityId, control: BodyControl2d) -> Result<(), PhysicsError> {
        let record = self.record(entity)?.clone();
        control.validate(entity, record.kind)?;
        let body = &mut self.backend.bodies[record.body];
        match control {
            BodyControl2d::Force([x, y]) => body.add_force(r2::Vector::new(x, y), true),
            BodyControl2d::Torque(value) => body.add_torque(value, true),
            BodyControl2d::LinearVelocity([x, y]) => body.set_linvel(r2::Vector::new(x, y), true),
            BodyControl2d::AngularVelocity(value) => {
                body.set_angvel(if body.is_rotation_locked() { 0.0 } else { value }, true);
            }
            BodyControl2d::Impulse([x, y]) => body.apply_impulse(r2::Vector::new(x, y), true),
            BodyControl2d::AngularImpulse(value) => body.apply_torque_impulse(value, true),
            BodyControl2d::ImpulseAtPoint { impulse: [x, y], point: [px, py] } => {
                body.apply_impulse_at_point(r2::Vector::new(x, y), r2::Vector::new(px, py), true);
            }
        }
        Ok(())
    }

    /// For callers that have validated an authored, not-yet-built body.
    /// Requests expire at the end of synchronization if it never materializes.
    pub fn remember_control(&mut self, entity: EntityId, kind: RigidBodyKind, control: BodyControl2d) -> Result<(), PhysicsError> {
        control.validate(entity, kind)?;
        self.pending_controls.entry(entity).or_default().push(control);
        Ok(())
    }

    /// Angular velocity in radians per second. Before materialization, returns
    /// the last queued setter; queued impulses are resolved when mass is known.
    pub fn angular_velocity(&self, entity: EntityId) -> Result<f32, PhysicsError> {
        if let Some(record) = self.bodies.get(&entity) {
            return Ok(self.backend.bodies[record.body].angvel());
        }
        self.pending_controls.get(&entity).and_then(|controls| {
            controls.iter().rev().find_map(|control| match control {
                BodyControl2d::AngularVelocity(value) => Some(*value),
                _ => None,
            })
        }).ok_or(PhysicsError::MissingEntity(entity))
    }

    /// Adds a world-space force for the next fixed step, then it clears.
    pub fn apply_force(&mut self, entity: EntityId, force: [f32; 2]) -> Result<(), PhysicsError> {
        self.apply_control(entity, BodyControl2d::Force(force))
    }

    /// Adds a torque for the next fixed step, then it clears.
    pub fn apply_torque(&mut self, entity: EntityId, torque: f32) -> Result<(), PhysicsError> {
        self.apply_control(entity, BodyControl2d::Torque(torque))
    }

    /// Sets radians per second on a dynamic or velocity-kinematic body.
    pub fn set_angular_velocity(&mut self, entity: EntityId, velocity: f32) -> Result<(), PhysicsError> {
        self.apply_control(entity, BodyControl2d::AngularVelocity(velocity))
    }

    /// Applies an immediate angular impulse, independent of fixed-step duration.
    pub fn apply_angular_impulse(&mut self, entity: EntityId, impulse: f32) -> Result<(), PhysicsError> {
        self.apply_control(entity, BodyControl2d::AngularImpulse(impulse))
    }

    /// Applies an immediate impulse at a world point, including its turning effect.
    pub fn apply_impulse_at_point(&mut self, entity: EntityId, impulse: [f32; 2], point: [f32; 2]) -> Result<(), PhysicsError> {
        self.apply_control(entity, BodyControl2d::ImpulseAtPoint { impulse, point })
    }

    pub(super) fn clear_step_forces(&mut self) {
        for record in self.bodies.values() {
            let body = &mut self.backend.bodies[record.body];
            body.reset_forces(false);
            body.reset_torques(false);
        }
    }
}
