//! What a Decay expression evaluates to, and the arithmetic over it.

use std::rc::Rc;

use decay_ir::Constant;
use decay_syntax::{BinaryOp, UnaryOp};

use crate::error::RuntimeError;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    /// Something the host owns, which a script may hold, compare, pass and
    /// store, but cannot construct, read into, or do arithmetic on.
    ///
    /// The number inside is the host's, and Decay attaches no meaning to it.
    /// That is the whole design: a script needs to be able to *name* another
    /// thing in the world in order to say anything about it, and it can do
    /// that without the language knowing what a world is. The engine packs an
    /// entity's slot and generation into it; a different host could pack
    /// something else, and nothing here would change.
    ///
    /// An absent reference is [`Value::Null`], not a reserved number, for the
    /// same reason an empty tile is null: every number is a real reference.
    Reference(u64),
    /// A list: several values, in order.
    ///
    /// Shared until changed, so passing one around or walking it copies
    /// nothing; a change to one that something else also holds copies it
    /// first, which is what makes a list a value.
    Array(Rc<Vec<Value>>),
    /// A map: values found by key, each key once, kept in the order the keys
    /// were first set. Shared until changed, like a list.
    ///
    /// A list of pairs rather than a hash table: its keys are the few values
    /// a script can compare — text, numbers, flags, variants, entities — and
    /// a map a scene's scripts keep holds tens of entries, where a search is
    /// as quick and the order is one a script can rely on.
    Map(Rc<Vec<(Value, Value)>>),
    /// A struct: its shape, and its fields' values in declared order. Shared
    /// until changed, like a list.
    Struct {
        shape: Rc<decay_ir::StructShape>,
        fields: Rc<Vec<Value>>,
    },
    /// Two numbers that travel together, `x` then `y`.
    ///
    /// A value like a number, copied rather than shared: `a = b; a.x = 1.0`
    /// leaves `b` alone, which is what anyone who has used a vector in a game
    /// expects and what a shared one would get wrong.
    Vec2([f64; 2]),
    /// Three numbers, `x`, `y`, `z`.
    Vec3([f64; 3]),
    /// A countdown: the seconds it has to go, never below zero, and the
    /// seconds it was started with. Copied like a number, and run down only
    /// by [`crate::ScriptInstance::advance_timers`] — the language has no
    /// clock of its own.
    Timer {
        left: f64,
        duration: f64,
    },
    /// An enum's variant, held by its full name: `Phase.Lobby`. Two are equal
    /// when they name the same variant of the same enum, and nothing else
    /// about one is visible to a script.
    Variant(Rc<str>),
    Null,
    Unit,
}

impl Value {
    /// The elements, for a value that holds several.
    #[must_use]
    pub fn elements(&self) -> Option<&Rc<Vec<Self>>> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    /// A vector's components, for a value that is one.
    #[must_use]
    pub fn components(&self) -> Option<&[f64]> {
        match self {
            Self::Vec2(components) => Some(components),
            Self::Vec3(components) => Some(components),
            _ => None,
        }
    }

    /// The vector made of these components, when there are two or three.
    #[must_use]
    pub fn vector(components: &[f64]) -> Option<Self> {
        match *components {
            [x, y] => Some(Self::Vec2([x, y])),
            [x, y, z] => Some(Self::Vec3([x, y, z])),
            _ => None,
        }
    }

    /// A collection of these values.
    #[must_use]
    pub fn array(values: Vec<Self>) -> Self {
        Self::Array(Rc::new(values))
    }
}

impl From<&Constant> for Value {
    fn from(value: &Constant) -> Self {
        match value {
            Constant::Number(value) => Self::Number(*value),
            Constant::String(value) => Self::String(value.clone()),
            Constant::Bool(value) => Self::Bool(*value),
            Constant::Null => Self::Null,
            Constant::Variant(name) => Self::Variant(Rc::from(name.as_str())),
        }
    }
}

pub(crate) fn apply_unary(op: UnaryOp, value: Value) -> Result<Value, RuntimeError> {
    match (op, value) {
        (UnaryOp::Negate, Value::Number(value)) => Ok(Value::Number(-value)),
        (UnaryOp::Negate, vector @ (Value::Vec2(_) | Value::Vec3(_))) => {
            crate::vector::map(&vector, |component| -component)
        }
        (UnaryOp::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
        _ => Err(RuntimeError::InvalidUnary),
    }
}

pub(crate) fn apply_binary(op: BinaryOp, left: Value, right: Value) -> Result<Value, RuntimeError> {
    if op == BinaryOp::Add
        && (matches!(left, Value::String(_)) || matches!(right, Value::String(_)))
    {
        return crate::text::join(&left, &right);
    }
    if left.components().is_some() || right.components().is_some() {
        return crate::vector::arithmetic(op, &left, &right);
    }
    match op {
        BinaryOp::Add => numbers(left, right, |a, b| a + b),
        BinaryOp::Subtract => numbers(left, right, |a, b| a - b),
        BinaryOp::Multiply => numbers(left, right, |a, b| a * b),
        BinaryOp::Divide => numbers(left, right, |a, b| a / b),
        // Remainder rather than a floored modulo, so the sign follows the left
        // operand as it does in the language Decay is shaped after. `% 0.0` is
        // NaN for the same reason `/ 0.0` is infinity: there is no integer
        // division here to trap.
        BinaryOp::Modulo => numbers(left, right, |a, b| a % b),
        BinaryOp::Less => compare(left, right, |a, b| a < b),
        BinaryOp::LessEqual => compare(left, right, |a, b| a <= b),
        BinaryOp::Greater => compare(left, right, |a, b| a > b),
        BinaryOp::GreaterEqual => compare(left, right, |a, b| a >= b),
        // Lowering does not emit these any more: `&&` and `||` became branches
        // so that a left operand which already decides the answer skips the
        // right one. They stay because `Instruction` is public and an IR built
        // by hand may still ask for the operation over two values it has
        // already evaluated, which is the one thing this can still mean.
        BinaryOp::And => booleans(left, right, |a, b| a && b),
        BinaryOp::Or => booleans(left, right, |a, b| a || b),
        BinaryOp::Equal => Ok(Value::Bool(left == right)),
        BinaryOp::NotEqual => Ok(Value::Bool(left != right)),
    }
}

pub(crate) fn numbers(
    left: Value,
    right: Value,
    op: impl FnOnce(f64, f64) -> f64,
) -> Result<Value, RuntimeError> {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(op(a, b))),
        _ => Err(RuntimeError::InvalidBinary),
    }
}

pub(crate) fn compare(
    left: Value,
    right: Value,
    op: impl FnOnce(f64, f64) -> bool,
) -> Result<Value, RuntimeError> {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(op(a, b))),
        _ => Err(RuntimeError::InvalidBinary),
    }
}

pub(crate) fn booleans(
    left: Value,
    right: Value,
    op: impl FnOnce(bool, bool) -> bool,
) -> Result<Value, RuntimeError> {
    match (left, right) {
        (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(op(a, b))),
        _ => Err(RuntimeError::InvalidBinary),
    }
}
