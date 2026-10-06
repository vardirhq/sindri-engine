//! A slider aligns local axes, constraining rotation and perpendicular motion.
use super::{OwnedJointSpec, PhysicsWorld2d};
use crate::{MotorMode2d, PhysicsError, SliderJoint2d};
use rapier2d::prelude as r2;

impl PhysicsWorld2d {
    /// Creates or edits an owned slider, preserving endpoint motion.
    ///
    /// # Errors
    /// Rejects invalid settings, identical endpoints or missing bodies atomically.
    pub fn set_slider_joint(
        &mut self,
        owner: sindri_core::EntityId,
        joint: SliderJoint2d,
    ) -> Result<(), PhysicsError> {
        joint.settings.validate()?;
        let s = joint.settings;
        let mut builder =
            r2::PrismaticJointBuilder::new(r2::Vector::new(s.first_axis[0], s.first_axis[1]))
                .local_axis2(r2::Vector::new(s.second_axis[0], s.second_axis[1]))
                .local_anchor1(r2::Vector::new(s.first_anchor[0], s.first_anchor[1]))
                .local_anchor2(r2::Vector::new(s.second_anchor[0], s.second_anchor[1]))
                .contacts_enabled(false);
        if s.limits_enabled {
            builder = builder.limits([s.lower_distance, s.upper_distance]);
        }
        if s.motor_enabled {
            builder = builder
                .motor_model(r2::MotorModel::ForceBased)
                .motor_max_force(s.motor_max_force);
            builder = match s.motor_mode {
                MotorMode2d::Velocity => builder.motor_velocity(s.motor_velocity, 1.0),
                MotorMode2d::Position => builder.motor_position(
                    s.motor_target_distance,
                    s.motor_stiffness,
                    s.motor_damping,
                ),
            };
        }
        self.set_owned_joint(owner, OwnedJointSpec::Slider(joint), builder.into())
    }
}
