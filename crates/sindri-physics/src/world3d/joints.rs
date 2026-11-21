//! Constraints owned by an entity, separate from body registration.
//!
//! The 3D counterpart of the 2D owned joints: each owner holds at most one
//! constraint, an unchanged request keeps the solver's constraint and its warm
//! start, and removing a body drops its constraints with it, so the owner's
//! next synchronization simply makes the constraint again.

use rapier3d::prelude as r3;
use sindri_core::EntityId;

use super::PhysicsWorld3d;
use crate::{DistanceJoint3d, PhysicsError};

#[derive(Clone)]
pub(super) struct OwnedJoint3d {
    handle: r3::ImpulseJointHandle,
    joint: DistanceJoint3d,
}

impl PhysicsWorld3d {
    /// Creates or replaces the rope owned by an entity.
    ///
    /// An unchanged request keeps the existing solver constraint. A change of
    /// length or anchors on the same endpoints retunes the constraint in place,
    /// keeping body motion; a change of endpoints replaces it. Invalid requests
    /// leave any existing constraint as it was.
    ///
    /// # Errors
    /// An invalid joint, or an endpoint with no body.
    pub fn set_distance_joint(
        &mut self,
        owner: EntityId,
        joint: DistanceJoint3d,
    ) -> Result<(), PhysicsError> {
        joint.validate()?;
        let first_body = self.record(joint.first)?.body;
        let second_body = self.record(joint.second)?.body;
        let data: r3::GenericJoint = r3::RopeJointBuilder::new(joint.max_distance)
            .local_anchor1(r3::Vector::from_array(joint.first_anchor))
            .local_anchor2(r3::Vector::from_array(joint.second_anchor))
            .contacts_enabled(false)
            .into();
        if let Some(record) = self.owned_joints.get_mut(&owner)
            && (record.joint.first, record.joint.second) == (joint.first, joint.second)
            && let Some(constraint) = self.backend.impulse_joints.get_mut(record.handle, true)
        {
            if record.joint != joint {
                constraint.data = data;
                record.joint = joint;
            }
            return Ok(());
        }
        self.remove_owned_joint(owner);
        let handle = self
            .backend
            .impulse_joints
            .insert(first_body, second_body, data, true);
        self.owned_joints
            .insert(owner, OwnedJoint3d { handle, joint });
        Ok(())
    }

    /// The rope an entity owns, while its constraint is live.
    #[must_use]
    pub fn distance_joint(&self, owner: EntityId) -> Option<DistanceJoint3d> {
        self.owned_joints
            .get(&owner)
            .filter(|record| self.backend.impulse_joints.get(record.handle).is_some())
            .map(|record| record.joint)
    }

    /// Removes only this owner's constraint, waking its endpoints.
    pub fn remove_owned_joint(&mut self, owner: EntityId) -> bool {
        let Some(record) = self.owned_joints.remove(&owner) else {
            return false;
        };
        self.backend
            .impulse_joints
            .remove(record.handle, true)
            .is_some()
    }
}
