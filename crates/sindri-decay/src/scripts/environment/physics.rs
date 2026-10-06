//! The 2D physics calls and their hit snapshot type.

use crate::surface::{ENTITY, PHYSICS, PHYSICS_CALLS, PhysicsCall};
use decay_semantic::{Environment, FunctionType, HostType, Type};

use crate::surface::contact::{CONTACT, CONTACT_FIELDS};
use crate::surface::raycast::{HIT_FIELDS, RAY_HIT};

pub(super) fn add_physics_surface(environment: &mut Environment) {
    add_snapshots(environment);
    let mut physics = HostType::new();
    for (name, call) in PHYSICS_CALLS {
        physics = physics.with_function(
            *name,
            FunctionType {
                params: physics_params(*call),
                return_type: physics_return_type(*call),
            },
        );
    }
    environment.add_type(PHYSICS, physics);
    environment.add_value(PHYSICS, Type::Named(PHYSICS.to_owned()));
}

fn physics_params(call: PhysicsCall) -> Vec<Type> {
    let entity = || Type::Named(ENTITY.to_owned());
    match call {
        // A ray's origin and direction, or a box's centre and half
        // size: two vectors and a number, then the filter.
        PhysicsCall::Raycast | PhysicsCall::OverlapBox => vec![
            Type::Vec2,
            Type::Vec2,
            Type::F32,
            Type::F32,
            Type::Bool,
            entity(),
        ],
        PhysicsCall::OverlapCircle => {
            vec![Type::Vec2, Type::F32, Type::F32, Type::Bool, entity()]
        }
        PhysicsCall::CastCircle => vec![
            Type::Vec2,
            Type::F32,
            Type::Vec2,
            Type::F32,
            Type::F32,
            Type::Bool,
            entity(),
        ],
        PhysicsCall::CastBox => vec![
            Type::Vec2,
            Type::Vec2,
            Type::F32,
            Type::Vec2,
            Type::F32,
            Type::F32,
            Type::Bool,
            entity(),
        ],
        PhysicsCall::SetContinuousCollision | PhysicsCall::SetJointEnabled => {
            vec![entity(), Type::Bool]
        }
        PhysicsCall::SetDistance
        | PhysicsCall::DropThrough
        | PhysicsCall::ApplyTorque
        | PhysicsCall::SetAngularVelocity
        | PhysicsCall::ApplyAngularImpulse => vec![entity(), Type::F32],
        PhysicsCall::ApplyForce => vec![entity(), Type::Vec2],
        PhysicsCall::ApplyImpulseAtPoint => vec![entity(), Type::Vec2, Type::Vec2],
        PhysicsCall::Layer => vec![Type::String],
        PhysicsCall::Mask => vec![Type::array_of(Type::String)],
        PhysicsCall::RemoveJoint
        | PhysicsCall::JointEnabled
        | PhysicsCall::ContinuousCollision
        | PhysicsCall::Contacts
        | PhysicsCall::VelocityX
        | PhysicsCall::VelocityY
        | PhysicsCall::AngularVelocity => vec![entity()],
        PhysicsCall::SetVelocity
        | PhysicsCall::ApplyImpulse
        | PhysicsCall::SetHingeMotor
        | PhysicsCall::SetSliderMotor => {
            vec![entity(), Type::F32, Type::F32]
        }
        PhysicsCall::SetSpring => vec![entity(), Type::F32, Type::F32, Type::F32],
        PhysicsCall::SetJointEndpoints => vec![entity(), entity(), entity()],
        PhysicsCall::ConnectDistance => {
            vec![entity(), entity(), Type::F32]
        }
        // An event query is about the entity the script is on, so
        // it takes nothing: an event is about a pair, and the pair
        // a script cares about is the one it is half of.
        _ => Vec::new(),
    }
}

fn physics_return_type(call: PhysicsCall) -> Type {
    let entity = || Type::Named(ENTITY.to_owned());
    match call {
        PhysicsCall::Contacts => Type::array_of(Type::Named(CONTACT.to_owned())),
        PhysicsCall::Raycast | PhysicsCall::CastCircle | PhysicsCall::CastBox => {
            Type::Named(RAY_HIT.to_owned())
        }
        PhysicsCall::VelocityX
        | PhysicsCall::VelocityY
        | PhysicsCall::Layer
        | PhysicsCall::Mask
        | PhysicsCall::AngularVelocity => Type::F32,
        PhysicsCall::ContinuousCollision | PhysicsCall::JointEnabled => Type::Bool,
        PhysicsCall::SetVelocity
        | PhysicsCall::ApplyImpulse
        | PhysicsCall::ConnectDistance
        | PhysicsCall::SetHingeMotor
        | PhysicsCall::SetSliderMotor
        | PhysicsCall::SetSpring
        | PhysicsCall::SetJointEnabled
        | PhysicsCall::SetDistance
        | PhysicsCall::SetJointEndpoints
        | PhysicsCall::RemoveJoint
        | PhysicsCall::SetContinuousCollision
        | PhysicsCall::DropThrough
        | PhysicsCall::ApplyForce
        | PhysicsCall::ApplyTorque
        | PhysicsCall::SetAngularVelocity
        | PhysicsCall::ApplyAngularImpulse
        | PhysicsCall::ApplyImpulseAtPoint => Type::Unit,
        _ => Type::array_of(entity()),
    }
}

fn add_snapshots(environment: &mut Environment) {
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
    let fields = vec![
        (CONTACT_FIELDS[0].to_owned(), entity()),
        (CONTACT_FIELDS[1].to_owned(), Type::Vec2),
        (CONTACT_FIELDS[2].to_owned(), Type::Vec2),
        (CONTACT_FIELDS[3].to_owned(), Type::F32),
        (CONTACT_FIELDS[4].to_owned(), Type::F32),
        (CONTACT_FIELDS[5].to_owned(), Type::Vec2),
    ];
    environment.add_struct(CONTACT, fields.clone());
    let mut contact = HostType::new();
    for (name, ty) in fields {
        contact = contact.with_value(name, ty);
    }
    environment.add_type(CONTACT, contact);
}
