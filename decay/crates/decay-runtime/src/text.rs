//! Text: joining it, spelling other values as it, and what it can be asked.
//!
//! Positions and lengths count characters, not bytes, so a script never sees
//! half of one.

use decay_syntax::StringOp;

use crate::error::RuntimeError;
use crate::runtime::describe;
use crate::value::Value;

/// The longest text a script may make, in bytes.
///
/// Joining doubles what a loop can hold on every pass, so without a ceiling
/// `s = s + s` thirty times over asks for a gigabyte. The operation budget
/// counts a join as one step whatever its size, so this is what bounds the
/// memory instead: generous for anything a game shows, and far below what
/// could hurt the process running the editor.
pub const TEXT_LIMIT: usize = 64 * 1024;

fn bounded(text: String) -> Result<Value, RuntimeError> {
    if text.len() > TEXT_LIMIT {
        return Err(RuntimeError::TextTooLong { limit: TEXT_LIMIT });
    }
    Ok(Value::String(text))
}

/// A number as a script expects to read it: `3`, not `3.0`; `0.5`; `-2`.
#[must_use]
pub fn spell_number(number: f64) -> String {
    if number == 0.0 {
        // `-0` would be a surprise on a scoreboard.
        return "0".to_owned();
    }
    if number.is_finite() && number.fract() == 0.0 && number.abs() < 1e15 {
        return format!("{number:.0}");
    }
    // Numbers come from `f32` sources, so their `f64` form carries digits
    // nobody wrote: `0.1` would spell as `0.10000000149011612`. The shortest
    // `f32` spelling is the one that reads back as what the script had.
    #[allow(clippy::cast_possible_truncation)]
    let narrow = number as f32;
    // Exact on purpose: the question is whether nothing was lost.
    #[allow(clippy::float_cmp)]
    let exact = f64::from(narrow) == number;
    if exact {
        format!("{narrow}")
    } else {
        format!("{number}")
    }
}

/// How a value is written into text by `+`.
fn spell(value: &Value) -> Result<String, RuntimeError> {
    Ok(match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => spell_number(*number),
        Value::Bool(flag) => flag.to_string(),
        // A variant by its own name: `Lobby`, which is what a label wants.
        Value::Variant(name) => name
            .split_once('.')
            .map_or(&**name, |(_, variant)| variant)
            .to_owned(),
        Value::Vec2(_) | Value::Vec3(_) => {
            let components = value.components().unwrap_or_default();
            let parts = components
                .iter()
                .map(|component| spell_number(*component))
                .collect::<Vec<_>>();
            format!("({})", parts.join(", "))
        }
        other => return Err(RuntimeError::NotText(describe(other))),
    })
}

/// A struct as `print` shows it: `Card(name: "Arc", weight: 1)`, a field
/// that is itself a struct shown the same way.
#[must_use]
pub fn show_struct(value: &Value) -> String {
    let Value::Struct { shape, fields } = value else {
        return spell(value).unwrap_or_else(|_| describe(value));
    };
    let parts = shape
        .fields
        .iter()
        .zip(fields.iter())
        .map(|(name, field)| {
            let shown = match field {
                Value::String(text) => format!("{text:?}"),
                Value::Struct { .. } => show_struct(field),
                Value::Array(values) => format!("{} entries", values.len()),
                Value::Reference(_) => "entity".to_owned(),
                Value::Null => "null".to_owned(),
                other => spell(other).unwrap_or_else(|_| describe(other)),
            };
            format!("{name}: {shown}")
        })
        .collect::<Vec<_>>();
    format!("{}({})", shape.name, parts.join(", "))
}

/// `left + right` where either side is text.
pub(crate) fn join(left: &Value, right: &Value) -> Result<Value, RuntimeError> {
    let mut joined = spell(left)?;
    joined.push_str(&spell(right)?);
    bounded(joined)
}

fn text_of(value: &Value) -> Result<&str, RuntimeError> {
    match value {
        Value::String(text) => Ok(text),
        other => Err(RuntimeError::NotText(describe(other))),
    }
}

/// A character position from a script's number: whole, and held to the text.
fn position(value: &Value, length: usize) -> Result<usize, RuntimeError> {
    let Value::Number(number) = value else {
        return Err(RuntimeError::IndexNotANumber(describe(value)));
    };
    if !number.is_finite() || number.fract() != 0.0 {
        return Err(RuntimeError::IndexNotWhole(*number));
    }
    if *number <= 0.0 {
        return Ok(0);
    }
    // Whole and positive, and held to the length below.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let position = *number as usize;
    Ok(position.min(length))
}

#[allow(clippy::cast_precision_loss)]
fn count(number: usize) -> Value {
    // Exact for any length text can reach under `TEXT_LIMIT`.
    Value::Number(number as f64)
}

/// A text property or method, applied to `text` with its arguments.
pub(crate) fn apply(op: StringOp, text: &Value, args: &[Value]) -> Result<Value, RuntimeError> {
    let text = text_of(text)?;
    let argument = |index: usize| {
        args.get(index)
            .ok_or(RuntimeError::StackUnderflow)
            .and_then(text_of)
    };
    Ok(match op {
        StringOp::Length => count(text.chars().count()),
        StringOp::Uppercase => Value::String(text.to_uppercase()),
        StringOp::Lowercase => Value::String(text.to_lowercase()),
        StringOp::Trimmed => Value::String(text.trim().to_owned()),
        StringOp::Contains => Value::Bool(text.contains(argument(0)?)),
        StringOp::StartsWith => Value::Bool(text.starts_with(argument(0)?)),
        StringOp::EndsWith => Value::Bool(text.ends_with(argument(0)?)),
        StringOp::Find => match text.find(argument(0)?) {
            Some(byte) => count(text[..byte].chars().count()),
            None => Value::Number(-1.0),
        },
        StringOp::Slice => {
            let length = text.chars().count();
            let start = position(args.first().ok_or(RuntimeError::StackUnderflow)?, length)?;
            let end = position(args.get(1).ok_or(RuntimeError::StackUnderflow)?, length)?;
            Value::String(
                text.chars()
                    .skip(start)
                    .take(end.saturating_sub(start))
                    .collect(),
            )
        }
        StringOp::Replace => {
            let old = argument(0)?;
            if old.is_empty() {
                // Replacing nothing would put `new` between every character,
                // which nobody means.
                Value::String(text.to_owned())
            } else {
                return bounded(text.replace(old, argument(1)?));
            }
        }
    })
}
