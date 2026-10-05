//! Constraint ownership and deferred lifecycle, separate from body registration.

use rapier2d::prelude as r2;
use sindri_core::EntityId;

use super::PhysicsWorld2d;
use crate::validate::positive;
use crate::{DistanceJoint2d, HingeJoint2d, PhysicsError};

pub(super) struct OwnedJoint {
    pub(super) handle: r2::ImpulseJointHandle,
    pub(super) joint: OwnedJointSpec,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum OwnedJointSpec {
    Distance(DistanceJoint2d),
    Hinge(HingeJoint2d),
}

impl OwnedJointSpec {
    pub(super) fn endpoints(self) -> (EntityId, EntityId) {
        match self {
            Self::Distance(joint) => (joint.first, joint.second),
            Self::Hinge(joint) => (joint.first, joint.second),
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
        let first = self.record(joint.first)?.body;
        let second = self.record(joint.second)?.body;
        if self.owned_joints.get(&owner).is_some_and(|record| {
            record.joint == OwnedJointSpec::Distance(joint)
                && self.backend.impulse_joints.get(record.handle).is_some()
        }) {
            return Ok(());
        }
        self.remove_owned_joint(owner);
        let handle = self.backend.impulse_joints.insert(
            first,
            second,
            r2::RopeJointBuilder::new(joint.max_distance).contacts_enabled(false),
            true,
        );
        self.owned_joints.insert(
            owner,
            OwnedJoint {
                handle,
                joint: OwnedJointSpec::Distance(joint),
            },
        );
        Ok(())
    }

    /// Creates or edits an owned hinge without resetting either body's motion.
    /// Settings edits between the same endpoints update the existing constraint
    /// and wake both bodies. Endpoints may be static or dynamic.
    ///
    /// # Errors
    /// Rejects self-connections, missing bodies or invalid settings before mutation.
    pub fn set_hinge_joint(
        &mut self,
        owner: EntityId,
        joint: HingeJoint2d,
    ) -> Result<(), PhysicsError> {
        if joint.first == joint.second {
            return Err(PhysicsError::JointToSelf(joint.first));
        }
        joint.settings.validate()?;
        let first = self.record(joint.first)?.body;
        let second = self.record(joint.second)?.body;
        let spec = OwnedJointSpec::Hinge(joint);
        if self.owned_joints.get(&owner).is_some_and(|record| {
            record.joint == spec && self.backend.impulse_joints.get(record.handle).is_some()
        }) {
            return Ok(());
        }
        let settings = joint.settings;
        let mut builder = r2::RevoluteJointBuilder::new()
            .local_anchor1(r2::Vector::new(
                settings.first_anchor[0],
                settings.first_anchor[1],
            ))
            .local_anchor2(r2::Vector::new(
                settings.second_anchor[0],
                settings.second_anchor[1],
            ))
            .contacts_enabled(false);
        if settings.limits_enabled {
            builder = builder.limits([settings.lower_angle, settings.upper_angle]);
        }
        if settings.motor_enabled {
            builder = builder
                .motor_model(r2::MotorModel::ForceBased)
                .motor_velocity(settings.motor_velocity, 1.0)
                .motor_max_force(settings.motor_max_torque);
        }
        if let Some(record) = self.owned_joints.get_mut(&owner)
            && matches!(record.joint, OwnedJointSpec::Hinge(_))
            && record.joint.endpoints() == (joint.first, joint.second)
            && let Some(constraint) = self.backend.impulse_joints.get_mut(record.handle, true)
        {
            constraint.data = builder.into();
            record.joint = spec;
            return Ok(());
        }
        self.remove_owned_joint(owner);
        let handle = self
            .backend
            .impulse_joints
            .insert(first, second, builder, true);
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
