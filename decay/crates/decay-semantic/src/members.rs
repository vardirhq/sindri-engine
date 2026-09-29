//! What the language's own values can be asked: each operation's parameters
//! and result, after the value it is asked of.
//!
//! One table, read by the analysis that checks a call, by an editor offering
//! completions, and by the generator that documents them, so none of the
//! three can say something the others do not.

use decay_syntax::{ColorOp, ListOp, MapOp, NumberOp, StringOp, TimerProperty, VectorOp};

use crate::types::{FunctionType, Type};

fn signature(params: Vec<Type>, return_type: Type) -> FunctionType {
    FunctionType {
        params,
        return_type,
    }
}

/// A vector operation on a vector of type `vector`. A property — one that
/// takes no arguments — has no parameters.
#[must_use]
pub fn vector_op_signature(op: VectorOp, vector: &Type) -> FunctionType {
    match op {
        VectorOp::Length => signature(vec![], Type::F32),
        VectorOp::Normalized => signature(vec![], vector.clone()),
        VectorOp::Dot | VectorOp::Distance => signature(vec![vector.clone()], Type::F32),
        VectorOp::Lerp => signature(vec![vector.clone(), Type::F32], vector.clone()),
    }
}

/// A text operation.
#[must_use]
pub fn string_op_signature(op: StringOp) -> FunctionType {
    match op {
        StringOp::Length => signature(vec![], Type::F32),
        StringOp::Uppercase | StringOp::Lowercase | StringOp::Trimmed => {
            signature(vec![], Type::String)
        }
        StringOp::Contains | StringOp::StartsWith | StringOp::EndsWith => {
            signature(vec![Type::String], Type::Bool)
        }
        StringOp::Find => signature(vec![Type::String], Type::F32),
        StringOp::Slice => signature(vec![Type::F32, Type::F32], Type::String),
        StringOp::Replace => signature(vec![Type::String, Type::String], Type::String),
    }
}

/// A number operation: each writes the number as text.
#[must_use]
pub fn number_op_signature(op: NumberOp) -> FunctionType {
    match op {
        NumberOp::Fixed | NumberOp::Padded => signature(vec![Type::F32], Type::String),
    }
}

/// A list operation on a list of `element`.
#[must_use]
pub fn list_op_signature(op: ListOp, element: &Type) -> FunctionType {
    match op {
        ListOp::Push => signature(vec![element.clone()], Type::Unit),
        ListOp::Pop => signature(vec![], element.clone()),
        ListOp::Insert => signature(vec![Type::F32, element.clone()], Type::Unit),
        ListOp::RemoveAt => signature(vec![Type::F32], element.clone()),
        ListOp::Clear => signature(vec![], Type::Unit),
        ListOp::Contains => signature(vec![element.clone()], Type::Bool),
        ListOp::IndexOf => signature(vec![element.clone()], Type::F32),
        ListOp::SetAt => signature(vec![Type::F32, element.clone()], element.clone()),
    }
}

/// A colour operation.
#[must_use]
pub fn color_op_signature(op: ColorOp) -> FunctionType {
    match op {
        ColorOp::Lerp => signature(vec![Type::Color, Type::F32], Type::Color),
        ColorOp::WithAlpha => signature(vec![Type::F32], Type::Color),
        ColorOp::FromHex => signature(vec![Type::String], Type::Color),
    }
}

/// A map operation on a map from `key` to `value`.
#[must_use]
pub fn map_op_signature(op: MapOp, key: &Type, value: &Type) -> FunctionType {
    match op {
        // `get(key, fallback)`, and `m[key] = value`, which gives the value.
        MapOp::Get | MapOp::Set => signature(vec![key.clone(), value.clone()], value.clone()),
        MapOp::Contains | MapOp::Remove => signature(vec![key.clone()], Type::Bool),
        MapOp::Keys => signature(vec![], Type::array_of(key.clone())),
        MapOp::Values => signature(vec![], Type::array_of(value.clone())),
        MapOp::Clear => signature(vec![], Type::Unit),
    }
}

/// What a timer's property is.
#[must_use]
pub const fn timer_property_type(property: TimerProperty) -> Type {
    match property {
        TimerProperty::Done => Type::Bool,
        TimerProperty::Left | TimerProperty::Duration | TimerProperty::Progress => Type::F32,
    }
}
