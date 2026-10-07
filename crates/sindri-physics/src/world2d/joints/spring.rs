//! A radial spring applies forces rather than enforcing a hard length bound.
use super::{OwnedJointSpec, PhysicsWorld2d};
use crate::{PhysicsError, SpringJoint2d};
use rapier2d::prelude as r2;

impl PhysicsWorld2d {
    /// Creates or edits a force-based owned spring, preserving endpoint motion.
    ///
    /// # Errors
    /// Rejects invalid settings, identical endpoints or missing bodies atomically.
    pub fn set_spring_joint(
        &mut self,
        owner: sindri_core::EntityId,
        joint: SpringJoint2d,
    ) -> Result<(), PhysicsError> {
        joint.settings.validate()?;
        let s = joint.settings;
        let builder = r2::SpringJointBuilder::new(s.rest_length, s.stiffness, s.damping)
            .spring_model(r2::MotorModel::ForceBased)
            .local_anchor1(r2::Vector::new(s.first_anchor[0], s.first_anchor[1]))
            .local_anchor2(r2::Vector::new(s.second_anchor[0], s.second_anchor[1]))
            .contacts_enabled(false);
        self.set_owned_joint(owner, OwnedJointSpec::Spring(joint), builder.into())
    }
}
