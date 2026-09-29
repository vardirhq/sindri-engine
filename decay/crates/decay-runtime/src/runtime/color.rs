//! A colour's operations: blending, changing its opacity, and reading one
//! from its hex spelling.

use decay_syntax::ColorOp;

use super::{Frame, describe};
use crate::error::RuntimeError;
use crate::value::Value;

pub(super) fn step(frame: &mut Frame, op: ColorOp) -> Result<(), RuntimeError> {
    let mut pop = || frame.stack.pop().ok_or(RuntimeError::StackUnderflow);
    let answer = match op {
        ColorOp::FromHex => {
            let Value::String(text) = pop()? else {
                return Err(RuntimeError::InvalidColor(
                    "something that is not text".to_owned(),
                ));
            };
            let channels = decay_syntax::parse_hex(&text)
                .ok_or_else(|| RuntimeError::InvalidColor(text.clone()))?;
            Value::Color(channels)
        }
        ColorOp::WithAlpha => {
            let alpha = number(&pop()?)?;
            let mut channels = channels(&pop()?)?;
            channels[3] = alpha;
            Value::Color(channels)
        }
        ColorOp::Lerp => {
            let t = number(&pop()?)?;
            let to = channels(&pop()?)?;
            let from = channels(&pop()?)?;
            let mut mixed = [0.0; 4];
            for ((slot, a), b) in mixed.iter_mut().zip(from).zip(to) {
                *slot = a + (b - a) * t;
            }
            Value::Color(mixed)
        }
    };
    frame.stack.push(answer);
    Ok(())
}

fn number(value: &Value) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(number) => Ok(*number),
        other => Err(RuntimeError::NotAVector(describe(other))),
    }
}

fn channels(value: &Value) -> Result<[f64; 4], RuntimeError> {
    match value {
        Value::Color(channels) => Ok(*channels),
        other => Err(RuntimeError::NotAVector(describe(other))),
    }
}
