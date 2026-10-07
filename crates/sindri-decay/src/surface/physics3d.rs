//! Dimension-specific typed controls; the legacy 2D surface remains separate.

use decay_semantic::{Environment, FunctionType, HostType, Type};

pub(crate) const PHYSICS3D: &str = "Physics3d";
pub(crate) const RAY_HIT: &str = "RayHit3d";
pub(crate) const HIT_FIELDS: [&str; 4] = ["entity", "point", "normal", "distance"];

#[derive(Clone, Copy)]
pub(crate) enum Physics3dCall {
    Layer,
    Mask,
    Raycast,
    OverlapSphere,
    CastSphere,
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
    ("layer", Physics3dCall::Layer),
    ("mask", Physics3dCall::Mask),
    ("raycast", Physics3dCall::Raycast),
    ("overlap_sphere", Physics3dCall::OverlapSphere),
    ("cast_sphere", Physics3dCall::CastSphere),
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
    let fields = vec![
        (HIT_FIELDS[0].to_owned(), entity()),
        (HIT_FIELDS[1].to_owned(), Type::Vec3),
        (HIT_FIELDS[2].to_owned(), Type::Vec3),
        (HIT_FIELDS[3].to_owned(), Type::F32),
    ];
    environment.add_struct(RAY_HIT, fields.clone());
    let mut hit = HostType::new();
    for (name, ty) in fields {
        hit = hit.with_value(name, ty);
    }
    environment.add_type(RAY_HIT, hit);
    let mut physics = HostType::new();
    for (name, call) in CALLS {
        let (params, return_type) = match call {
            Physics3dCall::Layer => (vec![Type::String], Type::F32),
            Physics3dCall::Mask => (vec![Type::array_of(Type::String)], Type::F32),
            Physics3dCall::Raycast | Physics3dCall::OverlapSphere | Physics3dCall::CastSphere => {
                query_signature(*call)
            }
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

fn query_signature(call: Physics3dCall) -> (Vec<Type>, Type) {
    let mut params = match call {
        Physics3dCall::Raycast => vec![Type::Vec3, Type::Vec3, Type::F32],
        Physics3dCall::OverlapSphere => vec![Type::Vec3, Type::F32],
        Physics3dCall::CastSphere => vec![Type::Vec3, Type::F32, Type::Vec3, Type::F32],
        _ => unreachable!("only query calls reach here"),
    };
    params.extend([
        Type::F32,
        Type::Bool,
        Type::Optional(Box::new(Type::Named(super::ENTITY.to_owned()))),
    ]);
    let result = if matches!(call, Physics3dCall::OverlapSphere) {
        Type::array_of(Type::Named(super::ENTITY.to_owned()))
    } else {
        Type::Optional(Box::new(Type::Named(RAY_HIT.to_owned())))
    };
    (params, result)
}
