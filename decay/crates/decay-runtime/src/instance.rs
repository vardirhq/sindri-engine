//! A script attached to something, and the fields it keeps between calls.

use std::collections::HashMap;

use crate::error::RuntimeError;
use crate::value::Value;

#[derive(Debug, Clone)]
pub(crate) struct Slot {
    pub(crate) value: Value,
    pub(crate) mutable: bool,
}

#[derive(Debug, Clone)]
pub struct ScriptInstance {
    pub(crate) container_name: String,
    pub(crate) fields: HashMap<String, Slot>,
}

impl ScriptInstance {
    #[must_use]
    pub fn container_name(&self) -> &str {
        &self.container_name
    }

    #[must_use]
    pub fn field(&self, name: &str) -> Option<&Value> {
        self.fields.get(name).map(|slot| &slot.value)
    }

    /// Runs down every timer this script's fields hold by `seconds`, stopping
    /// each at zero.
    ///
    /// The host's to call, when it decides time has passed: the language has
    /// no clock. A timer in a local is not run down — it lives for one call.
    pub fn advance_timers(&mut self, seconds: f64) {
        if seconds <= 0.0 || seconds.is_nan() {
            return;
        }
        for slot in self.fields.values_mut() {
            if let Value::Timer { left, .. } = &mut slot.value {
                *left = (*left - seconds).max(0.0);
            }
        }
    }

    /// Whether the script's own code may reassign this field: `var` rather
    /// than `let`. `None` for a field the script does not have.
    #[must_use]
    pub fn is_mutable(&self, name: &str) -> Option<bool> {
        self.fields.get(name).map(|slot| slot.mutable)
    }

    /// Sets a field from outside the script, as an authoring tool does.
    ///
    /// This deliberately ignores the field's mutability. `@export let speed`
    /// means *the author sets this and the script does not*, so the host
    /// writing it is the point rather than a violation — the immutability the
    /// analyzer enforces is immutability to the script's own code. Callers that
    /// want to honour `@export` should check
    /// [`decay_ir::IrField::exported`] before calling; the runtime does not
    /// know what a property panel is.
    ///
    /// Fails for a name the container does not declare, rather than adding one:
    /// a typo in an authored property is otherwise a value that silently goes
    /// nowhere.
    pub fn set_field(&mut self, name: &str, value: Value) -> Result<(), RuntimeError> {
        match self.fields.get_mut(name) {
            Some(slot) => {
                slot.value = value;
                Ok(())
            }
            None => Err(RuntimeError::UnknownPath(format!(
                "{}.{name}",
                self.container_name
            ))),
        }
    }

    /// Every field this instance holds, in declaration-independent order.
    pub fn fields(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.fields
            .iter()
            .map(|(name, slot)| (name.as_str(), &slot.value))
    }
}
