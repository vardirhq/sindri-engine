//! `Game.score`: a project's declared state, kept on the board.
//!
//! The board already held what scripts shared, by name, as numbers. A declared
//! field is kept there under the same name, so `Game.score` and an older
//! `Game.get("score", 0.0)` read one number while a project moves from one to
//! the other, and a test reading the board sees either. What the declaration
//! adds is that the name, the type and the starting value are written once
//! and checked everywhere.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};

use super::WorldHost;
use crate::scripts::{SharedField, board_key};

impl WorldHost<'_> {
    fn shared_field(&self, path: &Path) -> Option<(String, SharedField)> {
        let [state, field] = path.0.as_slice() else {
            return None;
        };
        let declared = self
            .peers
            .as_ref()?
            .project
            .states
            .get(state)?
            .get(field)?
            .clone();
        Some((board_key(state, field), declared))
    }

    /// `Game.score`: what the board holds, or the declared starting value.
    pub(super) fn shared_load(&self, path: &Path) -> Option<Value> {
        let (key, declared) = self.shared_field(path)?;
        Some(declared.value(self.blackboard.get(&key, declared.initial)))
    }

    /// `Game.score = 3.0`. The analyzer refuses a write to a `let`; this
    /// refuses one anyway, since a host is not obliged to trust its caller.
    pub(super) fn shared_store(
        &mut self,
        path: &Path,
        value: &Value,
    ) -> Result<bool, RuntimeError> {
        let Some((key, declared)) = self.shared_field(path) else {
            return Ok(false);
        };
        if !declared.mutable {
            return Err(RuntimeError::Host(format!(
                "`{}` is a `let` and cannot be changed",
                path.dotted()
            )));
        }
        let Some(number) = declared.number(value) else {
            return Err(RuntimeError::Host(format!(
                "`{}` cannot hold {value:?}",
                path.dotted()
            )));
        };
        self.blackboard.set(key, number);
        Ok(true)
    }
}
