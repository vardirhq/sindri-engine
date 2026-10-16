//! Transform methods share signatures between checking and execution.

use decay_semantic::{FunctionType, HostType, Type};

#[derive(Clone, Copy)]
pub(crate) enum TransformCall {
    LookAt,
    RotateAround,
}

pub(crate) const CALLS: &[(&str, TransformCall)] = &[
    ("look_at", TransformCall::LookAt),
    ("rotate_around", TransformCall::RotateAround),
];

pub(crate) fn add_calls(mut ty: HostType) -> HostType {
    for (name, call) in CALLS {
        ty = ty.with_function(
            *name,
            FunctionType {
                params: match call {
                    TransformCall::LookAt => vec![Type::Vec3],
                    TransformCall::RotateAround => vec![Type::Vec3, Type::Vec3, Type::F32],
                },
                return_type: Type::Unit,
            },
        );
    }
    ty
}
