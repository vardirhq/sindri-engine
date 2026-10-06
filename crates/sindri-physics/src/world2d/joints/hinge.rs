//! Building hinges; ownership and body lifetime stay in the parent module.
use super::{OwnedJointSpec, PhysicsWorld2d};
use crate::{HingeJoint2d, PhysicsError};
use rapier2d::prelude as r2;

impl PhysicsWorld2d {
    /// Creates or edits an owned hinge without resetting either body's motion.
    /// Settings edits between the same endpoints update the existing constraint
    /// and wake both bodies. Endpoints may be static or dynamic.
    ///
    /// # Errors
    /// Rejects self-connections, missing bodies or invalid settings before mutation.
    pub fn set_hinge_joint(
        &mut self,
        owner: sindri_core::EntityId,
        joint: HingeJoint2d,
    ) -> Result<(), PhysicsError> {
        if joint.first == joint.second {
            return Err(PhysicsError::JointToSelf(joint.first));
        }
        joint.settings.validate()?;
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
        self.set_owned_joint(owner, OwnedJointSpec::Hinge(joint), builder.into())
    }
}
