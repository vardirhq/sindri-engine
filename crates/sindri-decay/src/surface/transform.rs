//! Transform methods share signatures between checking and execution.

use decay_semantic::{FunctionType, HostType, Type};

#[derive(Clone, Copy)]
pub(crate) enum TransformCall {
    LookAt,
    RotateAround,
    ToLocal,
    ToWorld,
}

pub(crate) const CALLS: &[(&str, TransformCall)] = &[
    ("look_at", TransformCall::LookAt),
    ("rotate_around", TransformCall::RotateAround),
    ("to_local", TransformCall::ToLocal),
    ("to_world", TransformCall::ToWorld),
];

pub(crate) fn add_calls(mut ty: HostType) -> HostType {
    for (name, call) in CALLS {
        ty = ty.with_function(
            *name,
            FunctionType {
                params: match call {
                    TransformCall::LookAt | TransformCall::ToLocal | TransformCall::ToWorld => {
                        vec![Type::Vec3]
                    }
                    TransformCall::RotateAround => vec![Type::Vec3, Type::Vec3, Type::Number],
                },
                return_type: match call {
                    TransformCall::ToLocal | TransformCall::ToWorld => Type::Vec3,
                    TransformCall::LookAt | TransformCall::RotateAround => Type::Unit,
                },
            },
        );
    }
    ty
}
