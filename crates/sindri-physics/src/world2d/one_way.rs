//! Support-side contact filtering, including the backend's CCD path.

use std::collections::HashMap;

use rapier2d::prelude as r2;
use sindri_core::EntityId;

use super::PhysicsWorld2d;
use crate::validate::non_negative;
use crate::{OneWay2d, PhysicsError, RigidBodyKind};

// Allow a small penetration while maintaining support, in world units.
pub(super) const SUPPORT_SLOP: f32 = 0.05;

#[derive(Clone, Default)]
pub(super) struct OneWayHooks {
    pub policies: HashMap<r2::ColliderHandle, OneWay2d>,
    pub dropping: HashMap<r2::RigidBodyHandle, f32>,
}

impl PhysicsWorld2d {
    /// Applies a validated policy to every solid piece, without replacing the body.
    /// The normal is local to each piece; its rotation follows the body and piece.
    pub fn set_one_way(
        &mut self,
        entity: EntityId,
        policy: Option<OneWay2d>,
    ) -> Result<(), PhysicsError> {
        if let Some(policy) = policy {
            policy.validate()?;
        }
        let record = self.record(entity)?.clone();
        for handle in record.colliders {
            self.one_way.policies.remove(&handle);
            let collider = &mut self.backend.colliders[handle];
            if let Some(policy) = policy.filter(|_| !collider.is_sensor()) {
                self.one_way.policies.insert(handle, policy);
                collider.set_active_hooks(
                    r2::ActiveHooks::FILTER_CONTACT_PAIRS | r2::ActiveHooks::MODIFY_SOLVER_CONTACTS,
                );
            } else {
                collider.set_active_hooks(r2::ActiveHooks::empty());
            }
            // Rapier's hook setter does not mark a collider changed. Refresh its
            // unchanged pose so contacts are reconsidered and sleeping neighbors
            // wake even when this platform is static. Body and joints stay intact.
            collider.set_position(*collider.position());
        }
        self.backend.bodies[record.body].wake_up(true);
        Ok(())
    }

    /// Ignores only one-way solid contacts for this many simulation seconds.
    /// Zero cancels; a new request replaces the remaining duration.
    pub fn drop_through(&mut self, entity: EntityId, seconds: f32) -> Result<(), PhysicsError> {
        non_negative("drop_through_seconds", seconds)?;
        let record = self.record(entity)?.clone();
        if record.kind != RigidBodyKind::Dynamic {
            return Err(PhysicsError::WrongBodyKind(
                entity,
                "drop through",
                record.kind,
            ));
        }
        if seconds == 0.0 {
            self.one_way.dropping.remove(&record.body);
            self.pending_drop.remove(&entity);
        } else {
            self.one_way.dropping.insert(record.body, seconds);
        }
        self.backend.bodies[record.body].wake_up(true);
        Ok(())
    }

    /// The host has checked a freshly spawned authored dynamic body.
    /// The timer starts when that body materializes, not before.
    pub fn remember_drop_through(
        &mut self,
        entity: EntityId,
        seconds: f32,
    ) -> Result<(), PhysicsError> {
        non_negative("drop_through_seconds", seconds)?;
        if seconds == 0.0 {
            self.pending_drop.remove(&entity);
        } else {
            self.pending_drop.insert(entity, seconds);
        }
        Ok(())
    }
}

impl OneWayHooks {
    fn allows(
        &self,
        context: &r2::PairFilterContext<'_>,
        platform: r2::ColliderHandle,
        other: r2::ColliderHandle,
    ) -> bool {
        let Some(policy) = self.policies.get(&platform) else {
            return true;
        };
        let platform = &context.colliders[platform];
        let other = &context.colliders[other];
        if other
            .parent()
            .is_some_and(|body| self.dropping.contains_key(&body))
        {
            return false;
        }
        let local = r2::Vector::new(policy.normal[0], policy.normal[1])
            / policy.normal[0].hypot(policy.normal[1]);
        let normal = platform.position().rotation * local;
        let velocity = |collider: &r2::Collider| {
            collider.parent().map_or(r2::Vector::ZERO, |handle| {
                context.bodies[handle].velocity_at_point(collider.translation())
            })
        };
        // Reject ascent in the pair filter, which CCD calls before its sweep.
        if (velocity(other) - velocity(platform)).dot(normal) > 0.1 {
            return false;
        }
        let Some(front) = platform.shape().as_support_map() else {
            return false;
        };
        let Some(back) = other.shape().as_support_map() else {
            return false;
        };
        let top = front.support_point(platform.position(), normal);
        let bottom = back.support_point(other.position(), -normal);
        // A body already inside/from below must clear the support plane first.
        (bottom - top).dot(normal) >= -SUPPORT_SLOP
    }
}

impl r2::PhysicsHooks for OneWayHooks {
    fn filter_contact_pair(&self, context: &r2::PairFilterContext<'_>) -> Option<r2::SolverFlags> {
        (self.allows(context, context.collider1, context.collider2)
            && self.allows(context, context.collider2, context.collider1))
        .then_some(r2::SolverFlags::COMPUTE_RIGID_IMPULSES)
    }

    fn modify_solver_contacts(&self, context: &mut r2::ContactModificationContext<'_>) {
        for (platform, sign) in [(context.collider1, 1.0), (context.collider2, -1.0)] {
            if let Some(policy) = self.policies.get(&platform) {
                let local = r2::Vector::new(policy.normal[0], policy.normal[1])
                    / policy.normal[0].hypot(policy.normal[1]);
                let normal = context.colliders[platform].position().rotation * local;
                if let Some(rigid) = context.rigid_mut()
                    && (sign * *rigid.normal).dot(normal) < policy.angle.cos()
                {
                    rigid.solver_contacts.clear();
                }
            }
        }
    }
}
