//! Typed gameplay world calls.

use decay_semantic::{Environment, FunctionType, HostType, Type};

use crate::surface::{ENTITY, PREFAB, WORLD, WORLD_CALLS, WorldCall};

/// What a script can do to the world it is in: find, spawn, despawn, reparent,
/// and write an exported field on another script.
///
/// Its own function like every other namespace, rather than inline in
/// `environment`, so that adding a call does not grow one function towards the
/// limit the others were split out to stay under.
pub(super) fn add_world_surface(environment: &mut Environment) {
    let mut world = HostType::new();
    for (name, call) in WORLD_CALLS {
        world = world.with_function(
            *name,
            FunctionType {
                params: match call {
                    WorldCall::Find | WorldCall::WithTag | WorldCall::TakeSignal => {
                        vec![Type::String]
                    }
                    WorldCall::Nearest => vec![Type::String, Type::Vec3],
                    WorldCall::WithinRadius => vec![Type::String, Type::Vec3, Type::F32],
                    WorldCall::Spawn => vec![Type::Named(PREFAB.to_owned())],
                    WorldCall::SpawnChild => vec![
                        Type::Named(PREFAB.to_owned()),
                        Type::Named(ENTITY.to_owned()),
                    ],
                    WorldCall::Despawn | WorldCall::Exists | WorldCall::IsActive => {
                        vec![Type::Named(ENTITY.to_owned())]
                    }
                    WorldCall::SetActive => {
                        vec![Type::Named(ENTITY.to_owned()), Type::Bool]
                    }
                    WorldCall::HasTag => {
                        vec![Type::Named(ENTITY.to_owned()), Type::String]
                    }
                    WorldCall::SetParent => vec![
                        Type::Named(ENTITY.to_owned()),
                        Type::Named(ENTITY.to_owned()),
                    ],
                    WorldCall::SetShapePoint => vec![Type::F32, Type::F32, Type::F32],
                    // The value is `Unknown` because an exported field may be a
                    // number, a truth, or text, and Decay has no union. The
                    // host checks what it was given, and the instance refuses a
                    // value the field cannot hold when it is built.
                    WorldCall::SetProperty => {
                        vec![Type::Named(ENTITY.to_owned()), Type::String, Type::Unknown]
                    }
                    WorldCall::PropertyNumber | WorldCall::SendSignal => {
                        vec![Type::Named(ENTITY.to_owned()), Type::String, Type::F32]
                    }
                },
                return_type: match call {
                    WorldCall::Find
                    | WorldCall::Spawn
                    | WorldCall::SpawnChild
                    | WorldCall::Nearest => Type::Named(ENTITY.to_owned()),
                    WorldCall::WithTag | WorldCall::WithinRadius => {
                        Type::array_of(Type::Named(ENTITY.to_owned()))
                    }
                    WorldCall::Despawn
                    | WorldCall::SetParent
                    | WorldCall::SetShapePoint
                    | WorldCall::SetProperty
                    | WorldCall::SendSignal
                    | WorldCall::SetActive => Type::Unit,
                    WorldCall::Exists | WorldCall::IsActive | WorldCall::HasTag => Type::Bool,
                    WorldCall::PropertyNumber | WorldCall::TakeSignal => Type::F32,
                },
            },
        );
    }
    environment.add_type(WORLD, world);
    environment.add_value(WORLD, Type::Named(WORLD.to_owned()));
}
