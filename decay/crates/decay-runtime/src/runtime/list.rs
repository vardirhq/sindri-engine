//! Lists: building one, asking one, and changing the one a variable holds.

use std::collections::HashMap;
use std::rc::Rc;

use decay_ir::Path;
use decay_syntax::ListOp;

use super::{Frame, describe};
use crate::error::RuntimeError;
use crate::instance::Slot;
use crate::value::Value;

/// The most elements a script may put in one list.
///
/// The operation budget counts a `push` as one step, and a loop that never
/// stops pushing is one budget away from holding a million values; this is
/// what bounds the memory instead. Far beyond any list a scene's scripts
/// keep, and far below what would hurt the editor. A list the host hands
/// back is not held to it.
pub const LIST_LIMIT: usize = 10_000;

/// A position in a list, from a script's number: whole and not negative.
fn position(index: &Value) -> Result<usize, RuntimeError> {
    let Value::Number(number) = index else {
        return Err(RuntimeError::IndexNotANumber(describe(index)));
    };
    if !number.is_finite() || number.fract() != 0.0 || *number < 0.0 {
        return Err(RuntimeError::IndexNotWhole(*number));
    }
    // Guarded above: finite, non-negative, and whole.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok(*number as usize)
}

fn within(index: usize, length: usize) -> Result<usize, RuntimeError> {
    if index < length {
        Ok(index)
    } else {
        Err(RuntimeError::IndexOutOfRange { index, length })
    }
}

fn room(values: &[Value]) -> Result<(), RuntimeError> {
    if values.len() >= LIST_LIMIT {
        return Err(RuntimeError::ListTooLong { limit: LIST_LIMIT });
    }
    Ok(())
}

#[allow(clippy::cast_precision_loss)]
fn count(number: usize) -> Value {
    // Exact for any position a list can reach.
    Value::Number(number as f64)
}

/// The instructions that build, ask or change a list or a map.
pub(super) fn step(
    fields: &mut HashMap<String, Slot>,
    frame: &mut Frame,
    instruction: &decay_ir::Instruction,
) -> Result<(), RuntimeError> {
    match instruction {
        decay_ir::Instruction::MakeList(count) => make(frame, *count),
        decay_ir::Instruction::ListRead(op) => read(frame, *op),
        decay_ir::Instruction::ListChange {
            path,
            fields: inside,
            op,
        } => change(fields, frame, (path, inside), *op),
        // A map's are made alike, and kept beside it.
        _ => super::map::step(fields, frame, instruction),
    }
}

/// Pops `count` values, the last written on top, into the list of them.
fn make(frame: &mut Frame, count: usize) -> Result<(), RuntimeError> {
    if count > frame.stack.len() {
        return Err(RuntimeError::StackUnderflow);
    }
    let values = frame.stack.split_off(frame.stack.len() - count);
    frame.stack.push(Value::array(values));
    Ok(())
}

/// A question about a list: pops the arguments, then the list.
fn read(frame: &mut Frame, op: ListOp) -> Result<(), RuntimeError> {
    let wanted = frame.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
    let list = frame.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
    let values = list
        .elements()
        .ok_or_else(|| RuntimeError::NotACollection(describe(&list)))?;
    let found = values.iter().position(|value| *value == wanted);
    frame.stack.push(match op {
        ListOp::Contains => Value::Bool(found.is_some()),
        _ => found.map_or(Value::Number(-1.0), count),
    });
    Ok(())
}

/// The value a variable, parameter or field of this script holds, to change
/// in place: `xs` or `this.xs`.
pub(super) fn place<'v>(
    fields: &'v mut HashMap<String, Slot>,
    frame: &'v mut Frame,
    path: &Path,
) -> Result<&'v mut Value, RuntimeError> {
    let slot = match path.0.as_slice() {
        [name] => match frame.lookup_mut(name) {
            Some(slot) => Some(slot),
            None => fields.get_mut(name),
        },
        [this, name] if this == "this" => fields.get_mut(name),
        _ => None,
    };
    let slot = slot.ok_or_else(|| RuntimeError::UnknownPath(path.dotted()))?;
    if !slot.mutable {
        return Err(RuntimeError::Immutable(path.dotted()));
    }
    Ok(&mut slot.value)
}

/// The collection at `path`, down through the struct fields `inside` it, to
/// change in place: each struct copied only if it is shared, as a list or map
/// itself is.
pub(super) fn held_at<'v>(
    fields: &'v mut HashMap<String, Slot>,
    frame: &'v mut Frame,
    (path, inside): (&Path, &[usize]),
) -> Result<&'v mut Value, RuntimeError> {
    let mut held = place(fields, frame, path)?;
    for index in inside {
        let Value::Struct { fields, .. } = held else {
            return Err(RuntimeError::NotACollection(describe(held)));
        };
        held = Rc::make_mut(fields)
            .get_mut(*index)
            .ok_or(RuntimeError::StackUnderflow)?;
    }
    Ok(held)
}

/// A change to the list at `path`: pops the arguments, and pushes what the
/// change gives back.
fn change(
    fields: &mut HashMap<String, Slot>,
    frame: &mut Frame,
    (path, inside): (&Path, &[usize]),
    op: ListOp,
) -> Result<(), RuntimeError> {
    let mut args = Vec::with_capacity(op.arity());
    for _ in 0..op.arity() {
        args.push(frame.stack.pop().ok_or(RuntimeError::StackUnderflow)?);
    }
    args.reverse();
    let held = held_at(fields, frame, (path, inside))?;
    let Value::Array(list) = held else {
        return Err(RuntimeError::NotACollection(describe(held)));
    };
    // Copies the list only when something else still holds it too — a copy
    // assigned elsewhere, or a walk part way through it — which is what a
    // list being a value means.
    let values = Rc::make_mut(list);
    let mut args = args.into_iter();
    let mut next = || args.next().ok_or(RuntimeError::StackUnderflow);
    let result = match op {
        ListOp::Push => {
            room(values)?;
            values.push(next()?);
            Value::Unit
        }
        ListOp::Pop => values.pop().ok_or(RuntimeError::IndexOutOfRange {
            index: 0,
            length: 0,
        })?,
        ListOp::Insert => {
            let index = position(&next()?)?;
            if index > values.len() {
                return Err(RuntimeError::IndexOutOfRange {
                    index,
                    length: values.len(),
                });
            }
            room(values)?;
            values.insert(index, next()?);
            Value::Unit
        }
        ListOp::RemoveAt => {
            let index = within(position(&next()?)?, values.len())?;
            values.remove(index)
        }
        ListOp::Clear => {
            values.clear();
            Value::Unit
        }
        ListOp::SetAt => {
            let index = within(position(&next()?)?, values.len())?;
            let value = next()?;
            values[index] = value.clone();
            value
        }
        ListOp::Contains | ListOp::IndexOf => {
            unreachable!("a question is asked with ListRead, not made as a change")
        }
    };
    frame.stack.push(result);
    Ok(())
}
