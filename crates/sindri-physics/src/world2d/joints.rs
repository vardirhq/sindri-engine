//! Constraint ownership and deferred lifecycle, separate from body registration.

use rapier2d::prelude as r2;
use sindri_core::EntityId;

use super::PhysicsWorld2d;
use crate::validate::positive;
use crate::{DistanceJoint2d, HingeJoint2d, PhysicsError, SliderJoint2d, SpringJoint2d};

mod hinge;
mod slider;
mod spring;

pub(super) struct OwnedJoint {
    pub(super) handle: r2::ImpulseJointHandle,
    pub(super) joint: OwnedJointSpec,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum OwnedJointSpec {
    Distance(DistanceJoint2d),
    Hinge(HingeJoint2d),
    Slider(SliderJoint2d),
    Spring(SpringJoint2d),
}

impl OwnedJointSpec {
    pub(super) fn endpoints(self) -> (EntityId, EntityId) {
        match self {
            Self::Distance(joint) => (joint.first, joint.second),
            Self::Hinge(joint) => (joint.first, joint.second),
            Self::Slider(joint) => (joint.first, joint.second),
            Self::Spring(joint) => (joint.first, joint.second),
        }
    }
}

impl PhysicsWorld2d {
    /// Connects two bodies with a hard maximum-distance joint.
    ///
    /// The bodies may move closer and rotate freely, but the solver will not
    /// allow their centres to separate beyond `max_distance`. The backend uses
    /// Rapier's rope joint today; callers see only Sindri entities and units.
    pub fn connect_distance(
        &mut self,
        first: EntityId,
        second: EntityId,
        max_distance: f32,
    ) -> Result<(), PhysicsError> {
        validate_distance_joint(first, second, max_distance)?;
        let first_body = self.record(first)?.body;
        let second_body = self.record(second)?.body;
        self.backend.impulse_joints.insert(
            first_body,
            second_body,
            r2::RopeJointBuilder::new(max_distance).contacts_enabled(false),
            true,
        );
        Ok(())
    }

    /// Queues a distance joint whose bodies are authored but not both built yet.
    ///
    /// This is the joint equivalent of `remember_linear_velocity`: a prefab may
    /// spawn a chain and connect it in one script pass, while physics materializes
    /// all those bodies at the next scene synchronization.
    pub fn remember_distance_joint(
        &mut self,
        first: EntityId,
        second: EntityId,
        max_distance: f32,
    ) -> Result<(), PhysicsError> {
        validate_distance_joint(first, second, max_distance)?;
        self.pending_distance_joints
            .push(DistanceJoint2d::new(first, second, max_distance));
        Ok(())
    }

    /// Finishes the lifecycle window opened by scripts before synchronization.
    ///
    /// Every body has now had a chance to materialize. Pending joints whose two
    /// endpoints exist are created; requests whose endpoint vanished are simply
    /// discarded because there is no longer anything useful to connect.
    pub fn finish_synchronize(&mut self) -> Result<(), PhysicsError> {
        let pending = std::mem::take(&mut self.pending_distance_joints);
        for joint in pending {
            if self.contains(joint.first) && self.contains(joint.second) {
                self.connect_distance(joint.first, joint.second, joint.max_distance)?;
            }
        }
        self.pending_controls.clear();
        self.pending_drop.clear();
        self.pending_velocity.clear();
        Ok(())
    }

    /// Drops remembered work without resolving it.
    ///
    /// Kept for callers that deliberately abandon a synchronization pass.
    pub fn forget_pending(&mut self) {
        self.pending_controls.clear();
        self.pending_drop.clear();
        self.pending_velocity.clear();
        self.pending_distance_joints.clear();
    }

    /// Creates or replaces the constraint owned by an entity. Unchanged calls
    /// keep the existing solver constraint. Invalid edits leave it intact.
    /// Body state is preserved even when only the joint needs replacing.
    ///
    /// # Errors
    /// Returns an error for invalid distance, identical endpoints or missing bodies.
    pub fn set_distance_joint(
        &mut self,
        owner: EntityId,
        joint: DistanceJoint2d,
    ) -> Result<(), PhysicsError> {
        validate_distance_joint(joint.first, joint.second, joint.max_distance)?;
        self.set_owned_joint(
            owner,
            OwnedJointSpec::Distance(joint),
            r2::RopeJointBuilder::new(joint.max_distance)
                .contacts_enabled(false)
                .into(),
        )
    }

    /// The kind-specific caller validates settings before entering this seam.
    fn set_owned_joint(
        &mut self,
        owner: EntityId,
        spec: OwnedJointSpec,
        data: r2::GenericJoint,
    ) -> Result<(), PhysicsError> {
        let (first, second) = spec.endpoints();
        if first == second {
            return Err(PhysicsError::JointToSelf(first));
        }
        let first_body = self.record(first)?.body;
        let second_body = self.record(second)?.body;
        if self.owned_joints.get(&owner).is_some_and(|record| {
            record.joint == spec && self.backend.impulse_joints.get(record.handle).is_some()
        }) {
            return Ok(());
        }
        if let Some(record) = self.owned_joints.get_mut(&owner)
            && !matches!(spec, OwnedJointSpec::Distance(_))
            && std::mem::discriminant(&record.joint) == std::mem::discriminant(&spec)
            && record.joint.endpoints() == (first, second)
            && let Some(constraint) = self.backend.impulse_joints.get_mut(record.handle, true)
        {
            constraint.data = data;
            record.joint = spec;
            return Ok(());
        }
        self.remove_owned_joint(owner);
        let handle = self
            .backend
            .impulse_joints
            .insert(first_body, second_body, data, true);
        self.owned_joints.insert(
            owner,
            OwnedJoint {
                handle,
                joint: spec,
            },
        );
        Ok(())
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

fn validate_distance_joint(
    first: EntityId,
    second: EntityId,
    max_distance: f32,
) -> Result<(), PhysicsError> {
    if first == second {
        return Err(PhysicsError::JointToSelf(first));
    }
    positive("distance_joint_max_distance", max_distance)
}
