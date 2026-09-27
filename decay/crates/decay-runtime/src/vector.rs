//! What a vector does: its arithmetic, and what it can be asked.
//!
//! Components are `f64` like every other Decay number. Where a vector crosses
//! into an engine that stores `f32`, the host narrows it, exactly as it does a
//! number.

use decay_syntax::{BinaryOp, VectorOp};

use crate::error::RuntimeError;
use crate::value::Value;

/// A vector with `apply` done to each component.
pub(crate) fn map(vector: &Value, apply: impl Fn(f64) -> f64) -> Result<Value, RuntimeError> {
    let components = components(vector)?;
    let mapped: Vec<f64> = components.iter().map(|c| apply(*c)).collect();
    Ok(Value::vector(&mapped).unwrap_or(Value::Unit))
}

/// Two vectors of the same size combined component by component.
fn zip(
    left: &Value,
    right: &Value,
    apply: impl Fn(f64, f64) -> f64,
) -> Result<Value, RuntimeError> {
    let (a, b) = (components(left)?, components(right)?);
    if a.len() != b.len() {
        return Err(RuntimeError::InvalidBinary);
    }
    let zipped: Vec<f64> = a.iter().zip(b).map(|(x, y)| apply(*x, *y)).collect();
    Ok(Value::vector(&zipped).unwrap_or(Value::Unit))
}

/// `+`, `-`, `*` and `/` where at least one side is a vector.
///
/// Vectors add to and subtract from vectors of their own size, and scale by a
/// number from either side; a vector divides by a number. Everything else is
/// refused rather than guessed at: there is no one obvious meaning for a
/// vector times a vector, and a script that wants the dot product says so.
pub(crate) fn arithmetic(op: BinaryOp, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
    match (op, left, right) {
        (BinaryOp::Add, _, _) => zip(left, right, |a, b| a + b),
        (BinaryOp::Subtract, _, _) => zip(left, right, |a, b| a - b),
        (BinaryOp::Multiply, _, Value::Number(scale)) => map(left, |c| c * scale),
        (BinaryOp::Multiply, Value::Number(scale), _) => map(right, |c| c * scale),
        (BinaryOp::Divide, _, Value::Number(scale)) => map(left, |c| c / scale),
        (BinaryOp::Equal, _, _) => Ok(Value::Bool(left == right)),
        (BinaryOp::NotEqual, _, _) => Ok(Value::Bool(left != right)),
        _ => Err(RuntimeError::InvalidBinary),
    }
}

/// A vector property or method, given the vector and its arguments.
pub(crate) fn apply(op: VectorOp, vector: &Value, args: &[Value]) -> Result<Value, RuntimeError> {
    let length = |value: &Value| -> Result<f64, RuntimeError> {
        Ok(components(value)?.iter().map(|c| c * c).sum::<f64>().sqrt())
    };
    match op {
        VectorOp::Length => Ok(Value::Number(length(vector)?)),
        // A zero vector has no direction; answering zero rather than NaN keeps
        // `velocity.normalized * speed` standing still instead of poisoning
        // every number it touches.
        VectorOp::Normalized => {
            let size = length(vector)?;
            if size == 0.0 {
                map(vector, |_| 0.0)
            } else {
                map(vector, |c| c / size)
            }
        }
        VectorOp::Dot => {
            let other = argument(args, 0)?;
            let (a, b) = (components(vector)?, components(other)?);
            if a.len() != b.len() {
                return Err(RuntimeError::InvalidBinary);
            }
            Ok(Value::Number(a.iter().zip(b).map(|(x, y)| x * y).sum()))
        }
        VectorOp::Distance => {
            let difference = zip(vector, argument(args, 0)?, |a, b| a - b)?;
            Ok(Value::Number(length(&difference)?))
        }
        VectorOp::Lerp => {
            let Value::Number(t) = argument(args, 1)? else {
                return Err(RuntimeError::InvalidBinary);
            };
            let t = *t;
            zip(vector, argument(args, 0)?, |a, b| a + (b - a) * t)
        }
    }
}

/// One component, by position.
pub(crate) fn component(vector: &Value, index: usize) -> Result<Value, RuntimeError> {
    components(vector)?
        .get(index)
        .map(|c| Value::Number(*c))
        .ok_or_else(|| RuntimeError::NotAVector(format!("a vector without component {index}")))
}

/// The vector with one component replaced.
pub(crate) fn with_component(
    vector: &Value,
    index: usize,
    value: &Value,
) -> Result<Value, RuntimeError> {
    let Value::Number(number) = value else {
        return Err(RuntimeError::InvalidBinary);
    };
    let mut replaced = components(vector)?.to_vec();
    let slot = replaced
        .get_mut(index)
        .ok_or_else(|| RuntimeError::NotAVector(format!("a vector without component {index}")))?;
    *slot = *number;
    Ok(Value::vector(&replaced).unwrap_or(Value::Unit))
}

fn components(value: &Value) -> Result<&[f64], RuntimeError> {
    value
        .components()
        .ok_or_else(|| RuntimeError::NotAVector(super::runtime::describe(value)))
}

fn argument(args: &[Value], index: usize) -> Result<&Value, RuntimeError> {
    args.get(index).ok_or(RuntimeError::StackUnderflow)
}
