//! Ordered controls in the spawn-to-synchronization window.

use sindri_core::EntityId;

use super::{PhysicsWorld3d, validation::finite3};
use crate::{Collider3d, PhysicsError, RigidBody3d, RigidBodyKind};

/// World-space velocity setters and impulses, replayed in call order once mass
/// is known. Queuing never authors starting motion or inserts query geometry.
#[derive(Clone, Copy, Debug)]
pub enum BodyControl3d {
    LinearVelocity([f32; 3]),
    AngularVelocity([f32; 3]),
    Impulse([f32; 3]),
}

impl BodyControl3d {
    fn validate(self, entity: EntityId, kind: RigidBodyKind) -> Result<(), PhysicsError> {
        let (name, value) = match self {
            Self::LinearVelocity(value) => ("linear_velocity", value),
            Self::AngularVelocity(value) => ("angular_velocity", value),
            Self::Impulse(value) => ("impulse", value),
        };
        finite3(name, value)?;
        let velocity = !matches!(self, Self::Impulse(_));
        if kind != RigidBodyKind::Dynamic && !(velocity && kind == RigidBodyKind::KinematicVelocity)
        {
            return Err(PhysicsError::WrongBodyKind(entity, "motion control", kind));
        }
        Ok(())
    }
}

impl PhysicsWorld3d {
    /// Applies one validated control immediately to a live body.
    pub fn apply_control(
        &mut self,
        entity: EntityId,
        control: BodyControl3d,
    ) -> Result<(), PhysicsError> {
        match control {
            BodyControl3d::LinearVelocity(value) => self.set_linear_velocity(entity, value),
            BodyControl3d::AngularVelocity(value) => self.set_angular_velocity(entity, value),
            BodyControl3d::Impulse(value) => self.apply_impulse(entity, value),
        }
    }

    /// Queues a finite request for an authored body that is not yet registered.
    /// The caller supplies its validated kind; insertion checks that kind again.
    /// Live bodies must use [`Self::apply_control`] instead.
    pub fn remember_control(
        &mut self,
        entity: EntityId,
        kind: RigidBodyKind,
        control: BodyControl3d,
    ) -> Result<(), PhysicsError> {
        if self.contains(entity) {
            return Err(PhysicsError::EntityAlreadyRegistered(entity));
        }
        control.validate(entity, kind)?;
        self.pending_controls
            .entry(entity)
            .or_default()
            .push(control);
        Ok(())
    }

    /// Validates body, colliders and every queued control without mutation.
    /// Scene drivers call this across their complete batch before reconciling.
    pub fn validate_insertion(
        &self,
        entity: EntityId,
        body: RigidBody3d,
        colliders: &[Collider3d],
    ) -> Result<(), PhysicsError> {
        Self::validate_body(entity, body, colliders)?;
        if let Some(controls) = self.pending_controls.get(&entity) {
            for &control in controls {
                control.validate(entity, body.kind)?;
            }
        }
        Ok(())
    }

    /// Discards requests not consumed by a successful synchronization.
    /// Hosts call this after their whole validated insertion batch; failed
    /// prevalidation must retain pending requests for a corrected retry.
    pub fn finish_synchronize(&mut self) {
        self.pending_controls.clear();
    }

    /// Last queued linear setter, before impulses can resolve against mass.
    pub fn pending_linear_velocity(&self, entity: EntityId) -> Option<[f32; 3]> {
        self.pending_velocity(entity, false)
    }

    /// Last queued angular setter. Rotation locking is applied at insertion.
    pub fn pending_angular_velocity(&self, entity: EntityId) -> Option<[f32; 3]> {
        self.pending_velocity(entity, true)
    }

    fn pending_velocity(&self, entity: EntityId, angular: bool) -> Option<[f32; 3]> {
        self.pending_controls
            .get(&entity)?
            .iter()
            .rev()
            .find_map(|control| match (*control, angular) {
                (BodyControl3d::LinearVelocity(value), false)
                | (BodyControl3d::AngularVelocity(value), true) => Some(value),
                _ => None,
            })
    }
}
