//! What a script reads about the frame rather than the world: the time, and
//! the numbers that are the same everywhere.

use decay_runtime::Value;

use super::WorldHost;
use crate::surface::{CONSTANTS, TIME, TIME_VALUES, TimeValue};

impl WorldHost<'_> {
    /// `Time.delta`, and `PI` and the like: the frame and named numbers, never
    /// the world, so never about a subject — a reference cannot be asked for
    /// the time. An unknown `Time.` name is left for the rest to refuse.
    pub(super) fn time_or_constant(&self, parts: &[&str]) -> Option<Value> {
        match parts {
            [namespace, name] if *namespace == TIME => TIME_VALUES
                .iter()
                .find(|(known, _)| known == name)
                .map(|(_, value)| {
                    Value::Number(f64::from(match value {
                        TimeValue::Delta => self.context.delta_seconds,
                        TimeValue::Elapsed => self.context.elapsed_seconds,
                    }))
                }),
            [name] => CONSTANTS
                .iter()
                .find(|(known, _)| known == name)
                .map(|(_, number)| Value::Number(*number)),
            _ => None,
        }
    }
}
