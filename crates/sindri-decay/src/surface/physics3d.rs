//! Dimension-specific typed controls; the legacy 2D surface remains separate.

use decay_semantic::{Environment, FunctionType, HostType, Type};

pub(crate) const PHYSICS3D: &str = "Physics3d";

#[derive(Clone, Copy)]
pub(crate) enum Physics3dCall {
    Velocity,
    SetVelocity,
    AngularVelocity,
    SetAngularVelocity,
    ApplyImpulse,
    CollisionStarted,
    CollisionStopped,
    SensorEntered,
    SensorExited,
}

pub(crate) const CALLS: &[(&str, Physics3dCall)] = &[
    ("velocity", Physics3dCall::Velocity),
    ("set_velocity", Physics3dCall::SetVelocity),
    ("angular_velocity", Physics3dCall::AngularVelocity),
    ("set_angular_velocity", Physics3dCall::SetAngularVelocity),
    ("apply_impulse", Physics3dCall::ApplyImpulse),
    ("collision_started", Physics3dCall::CollisionStarted),
    ("collision_stopped", Physics3dCall::CollisionStopped),
    ("sensor_entered", Physics3dCall::SensorEntered),
    ("sensor_exited", Physics3dCall::SensorExited),
];

pub(crate) fn add_surface(environment: &mut Environment) {
    let entity = || Type::Named(super::ENTITY.to_owned());
    let mut physics = HostType::new();
    for (name, call) in CALLS {
        let (params, return_type) = match call {
            Physics3dCall::Velocity | Physics3dCall::AngularVelocity => {
                (vec![entity()], Type::Vec3)
            }
            Physics3dCall::SetVelocity
            | Physics3dCall::SetAngularVelocity
            | Physics3dCall::ApplyImpulse => (vec![entity(), Type::Vec3], Type::Unit),
            _ => (vec![], Type::array_of(entity())),
        };
        physics = physics.with_function(
            *name,
            FunctionType {
                params,
                return_type,
            },
        );
    }
    environment.add_type(PHYSICS3D, physics);
    environment.add_value(PHYSICS3D, Type::Named(PHYSICS3D.to_owned()));
}
