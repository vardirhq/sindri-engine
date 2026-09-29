//! Reading a collection: an element or entry by index or key, its length, and
//! walking it with `for`.

use decay_ir::Instruction;

use super::{Frame, Runtime, Walk, describe, map};
use crate::error::RuntimeError;
use crate::host::Host;
use crate::value::Value;

impl<H: Host> Runtime<'_, H> {
    /// The element an index names, refusing every way an index can be wrong.
    ///
    /// There is no integer type, so "a whole number" is a runtime property of
    /// the value rather than a static property of its type. Each failure is
    /// named separately because they are different mistakes: `items[1.5]` is a
    /// calculation that should have rounded, and `items[9]` on a collection of
    /// three is a loop bound that is wrong.
    pub(super) fn element_at(object: &Value, index: &Value) -> Result<Value, RuntimeError> {
        if let Value::Map(entries) = object {
            return map::entry(entries, index);
        }
        let values = object
            .elements()
            .ok_or_else(|| RuntimeError::NotACollection(describe(object)))?;
        let Value::Number(number) = index else {
            return Err(RuntimeError::IndexNotANumber(describe(index)));
        };
        if !number.is_finite() || number.fract() != 0.0 || *number < 0.0 {
            return Err(RuntimeError::IndexNotWhole(*number));
        }
        // Guarded above: finite, non-negative, and whole.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let position = *number as usize;
        values
            .get(position)
            .cloned()
            .ok_or(RuntimeError::IndexOutOfRange {
                index: position,
                length: values.len(),
            })
    }

    /// The instructions that work on a collection.
    ///
    /// Their own function because they are the one group that shares a failure
    /// vocabulary, and because the evaluation loop is easier to read as a list
    /// of what an instruction *is* than as a list of what each one does.
    ///
    /// Answers with a jump target when the instruction takes one, which only
    /// the exhaustion of a walk does.
    pub(super) fn step_collection(
        frame: &mut Frame,
        instruction: &Instruction,
        length: usize,
    ) -> Result<Option<usize>, RuntimeError> {
        match instruction {
            Instruction::Index => {
                let index = frame.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                let object = frame.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                frame.stack.push(Self::element_at(&object, &index)?);
            }
            Instruction::Length => {
                let object = frame.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                if let Value::Map(entries) = &object {
                    #[allow(clippy::cast_precision_loss)]
                    frame.stack.push(Value::Number(entries.len() as f64));
                    return Ok(None);
                }
                let values = object
                    .elements()
                    .ok_or_else(|| RuntimeError::NotACollection(describe(&object)))?;
                // `usize` to `f64` is exact for every length a collection can
                // reach here, and a host bounds those far below the point
                // where it would not be.
                #[allow(clippy::cast_precision_loss)]
                frame.stack.push(Value::Number(values.len() as f64));
            }
            Instruction::IterBegin => {
                let object = frame.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                let over = object
                    .elements()
                    .ok_or_else(|| RuntimeError::NotACollection(describe(&object)))?
                    .clone();
                frame.walks.push(Walk::Elements { over, next: 0 });
            }
            Instruction::IterRange => {
                let end = frame.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                let start = frame.stack.pop().ok_or(RuntimeError::StackUnderflow)?;
                let (Value::Number(start), Value::Number(end)) = (&start, &end) else {
                    return Err(RuntimeError::IndexNotANumber(describe(&start)));
                };
                frame.walks.push(Walk::Range {
                    next: *start,
                    end: *end,
                });
            }
            Instruction::IterNext(target) => {
                let walk = frame.walks.last_mut().ok_or(RuntimeError::StackUnderflow)?;
                let value = match walk {
                    Walk::Elements { over, next } => {
                        let value = over.get(*next).cloned();
                        *next += 1;
                        value
                    }
                    Walk::Range { next, end } => (*next < *end).then(|| {
                        let value = Value::Number(*next);
                        *next += 1.0;
                        value
                    }),
                };
                let Some(value) = value else {
                    frame.walks.pop();
                    if *target > length {
                        return Err(RuntimeError::InvalidJump(*target));
                    }
                    return Ok(Some(*target));
                };
                frame.stack.push(value);
            }
            Instruction::IterEnd => {
                frame.walks.pop().ok_or(RuntimeError::StackUnderflow)?;
            }
            _ => unreachable!("only the collection instructions reach here"),
        }
        Ok(None)
    }
}
