//! The 2D physics calls and their hit snapshot type.

use crate::surface::{ENTITY, PHYSICS, PHYSICS_CALLS, PhysicsCall};
use decay_semantic::{Environment, FunctionType, HostType, Type};

use crate::surface::raycast::{HIT_FIELDS, RAY_HIT};

pub(super) fn add_physics_surface(environment: &mut Environment) {
    let entity = || Type::Named(ENTITY.to_owned());
    let fields = vec![
        (HIT_FIELDS[0].to_owned(), entity()),
        (HIT_FIELDS[1].to_owned(), Type::Vec2),
        (HIT_FIELDS[2].to_owned(), Type::Vec2),
        (HIT_FIELDS[3].to_owned(), Type::F32),
    ];
    environment.add_struct(RAY_HIT, fields.clone());
    let mut hit = HostType::new();
    for (name, ty) in fields {
        hit = hit.with_value(name, ty);
    }
    environment.add_type(RAY_HIT, hit);
    let mut physics = HostType::new();
    for (name, call) in PHYSICS_CALLS {
        physics = physics.with_function(
            *name,
            FunctionType {
                params: match call {
                    PhysicsCall::Raycast => vec![
                        Type::Vec2,
                        Type::Vec2,
                        Type::F32,
                        Type::F32,
                        Type::Bool,
                        entity(),
                    ],
                    PhysicsCall::VelocityX | PhysicsCall::VelocityY => vec![entity()],
                    PhysicsCall::SetVelocity | PhysicsCall::ApplyImpulse => {
                        vec![entity(), Type::F32, Type::F32]
                    }
                    PhysicsCall::ConnectDistance => {
                        vec![entity(), entity(), Type::F32]
                    }
                    // An event query is about the entity the script is on, so
                    // it takes nothing: an event is about a pair, and the pair
                    // a script cares about is the one it is half of.
                    _ => Vec::new(),
                },
                return_type: match call {
                    PhysicsCall::Raycast => Type::Named(RAY_HIT.to_owned()),
                    PhysicsCall::VelocityX | PhysicsCall::VelocityY => Type::F32,
                    PhysicsCall::SetVelocity
                    | PhysicsCall::ApplyImpulse
                    | PhysicsCall::ConnectDistance => Type::Unit,
                    _ => Type::array_of(entity()),
                },
            },
        );
    }
    environment.add_type(PHYSICS, physics);
    environment.add_value(PHYSICS, Type::Named(PHYSICS.to_owned()));
}
