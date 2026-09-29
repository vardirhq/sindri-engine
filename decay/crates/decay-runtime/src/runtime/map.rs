//! Maps: building one, asking one, and changing the one a variable holds.

use std::collections::HashMap;
use std::rc::Rc;

use decay_syntax::MapOp;

use super::list::{LIST_LIMIT, held_at};
use super::{Frame, describe};
use crate::error::RuntimeError;
use crate::instance::Slot;
use crate::value::Value;

/// The most keys a script may put in one map: the same bound as a list's,
/// for the same reason.
pub const MAP_LIMIT: usize = LIST_LIMIT;

/// The instructions that build, ask or change a map.
pub(super) fn step(
    fields: &mut HashMap<String, Slot>,
    frame: &mut Frame,
    instruction: &decay_ir::Instruction,
) -> Result<(), RuntimeError> {
    match instruction {
        decay_ir::Instruction::MakeMap(count) => make(frame, *count),
        decay_ir::Instruction::MapRead(op) => read(frame, *op),
        decay_ir::Instruction::MapChange {
            path,
            fields: inside,
            op,
        } => change(fields, frame, (path, inside), *op),
        _ => unreachable!("only the map instructions reach here"),
    }
}

/// The value at `key`, for `map[key]`: a key the map does not have is a
/// mistake, as an index past a list's end is. `get` is for a key that may
/// not be there.
pub(super) fn entry(entries: &[(Value, Value)], key: &Value) -> Result<Value, RuntimeError> {
    entries
        .iter()
        .find(|(known, _)| known == key)
        .map(|(_, value)| value.clone())
        .ok_or_else(|| RuntimeError::MissingKey(shown(key)))
}

/// A key as a message names it: its spelling where it has one.
fn shown(key: &Value) -> String {
    match key {
        Value::String(text) => format!("{text:?}"),
        Value::Number(number) => crate::text::spell_number(*number),
        Value::Bool(flag) => flag.to_string(),
        Value::Variant(name) => name.to_string(),
        other => describe(other),
    }
}

/// Pops `count` key-then-value pairs into the map of them. A key written
/// twice keeps its last value and its first place.
fn make(frame: &mut Frame, count: usize) -> Result<(), RuntimeError> {
    if count * 2 > frame.stack.len() {
        return Err(RuntimeError::StackUnderflow);
    }
    let flat = frame.stack.split_off(frame.stack.len() - count * 2);
    let mut entries: Vec<(Value, Value)> = Vec::with_capacity(count);
    let mut values = flat.into_iter();
    while let (Some(key), Some(value)) = (values.next(), values.next()) {
        set(&mut entries, key, value)?;
    }
    frame.stack.push(Value::Map(Rc::new(entries)));
    Ok(())
}

fn set(entries: &mut Vec<(Value, Value)>, key: Value, value: Value) -> Result<(), RuntimeError> {
    if let Some((_, held)) = entries.iter_mut().find(|(known, _)| *known == key) {
        *held = value;
        return Ok(());
    }
    if entries.len() >= MAP_LIMIT {
        return Err(RuntimeError::ListTooLong { limit: MAP_LIMIT });
    }
    entries.push((key, value));
    Ok(())
}

/// A question about a map: pops the arguments, then the map.
fn read(frame: &mut Frame, op: MapOp) -> Result<(), RuntimeError> {
    let mut args = Vec::with_capacity(op.arity());
    for _ in 0..op.arity() {
        args.push(frame.stack.pop().ok_or(RuntimeError::StackUnderflow)?);
    }
    args.reverse();
    let map = frame.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
    let Value::Map(entries) = &map else {
        return Err(RuntimeError::NotACollection(describe(&map)));
    };
    let mut args = args.into_iter();
    let mut next = || args.next().ok_or(RuntimeError::StackUnderflow);
    let answer = match op {
        MapOp::Get => {
            let key = next()?;
            let fallback = next()?;
            entries
                .iter()
                .find(|(known, _)| *known == key)
                .map_or(fallback, |(_, value)| value.clone())
        }
        MapOp::Contains => {
            let key = next()?;
            Value::Bool(entries.iter().any(|(known, _)| *known == key))
        }
        MapOp::Keys => Value::array(entries.iter().map(|(key, _)| key.clone()).collect()),
        MapOp::Values => Value::array(entries.iter().map(|(_, value)| value.clone()).collect()),
        MapOp::Remove | MapOp::Clear | MapOp::Set => {
            unreachable!("a change is made with MapChange, not asked as a question")
        }
    };
    frame.stack.push(answer);
    Ok(())
}

/// A change to the map at `path`: pops the arguments, and pushes what the
/// change gives back.
fn change(
    fields: &mut HashMap<String, Slot>,
    frame: &mut Frame,
    (path, inside): (&decay_ir::Path, &[usize]),
    op: MapOp,
) -> Result<(), RuntimeError> {
    let mut args = Vec::with_capacity(op.arity());
    for _ in 0..op.arity() {
        args.push(frame.stack.pop().ok_or(RuntimeError::StackUnderflow)?);
    }
    args.reverse();
    let held = held_at(fields, frame, (path, inside))?;
    let Value::Map(map) = held else {
        return Err(RuntimeError::NotACollection(describe(held)));
    };
    // Copies the map only when something else still holds it too, which is
    // what a map being a value means.
    let entries = Rc::make_mut(map);
    let mut args = args.into_iter();
    let mut next = || args.next().ok_or(RuntimeError::StackUnderflow);
    let result = match op {
        MapOp::Set => {
            let key = next()?;
            let value = next()?;
            set(entries, key, value.clone())?;
            value
        }
        MapOp::Remove => {
            let key = next()?;
            let before = entries.len();
            entries.retain(|(known, _)| *known != key);
            Value::Bool(entries.len() != before)
        }
        MapOp::Clear => {
            entries.clear();
            Value::Unit
        }
        MapOp::Get | MapOp::Contains | MapOp::Keys | MapOp::Values => {
            unreachable!("a question is asked with MapRead, not made as a change")
        }
    };
    frame.stack.push(result);
    Ok(())
}
