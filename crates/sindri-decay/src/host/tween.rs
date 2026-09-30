//! Managed values; scripts apply them through ordinary checked write paths.
use super::WorldHost;
use crate::{
    surface::tween::{CALLS, METHODS, TweenCall},
    tweens::{Track, Tweens},
};
use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::Easing;

fn error(path: &Path, message: &str) -> RuntimeError {
    RuntimeError::Host(format!("{}: {message}", path.dotted()))
}

fn finite(value: &Value) -> bool {
    match value {
        Value::Number(value) => value.is_finite(),
        value => value
            .components()
            .is_some_and(|values| values.iter().all(|v| v.is_finite())),
    }
}

impl WorldHost<'_> {
    fn tween_store(&mut self, path: &Path) -> Result<&mut Tweens, RuntimeError> {
        self.tweens
            .as_deref_mut()
            .ok_or_else(|| error(path, "requires the script runner's tween service"))
    }

    pub(super) fn tween_create(
        &mut self,
        call: TweenCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let [from, to, Value::Number(duration), Value::String(curve)] = args else {
            return Err(error(
                path,
                "takes from, to, duration in seconds and an easing name",
            ));
        };
        if !call.accepts(from) || !call.accepts(to) || !finite(from) || !finite(to) {
            return Err(error(
                path,
                "requires finite endpoints of the factory's type",
            ));
        }
        if !duration.is_finite() || *duration < 0.0 {
            return Err(error(path, "duration must be finite and non-negative"));
        }
        let easing = Easing::named(curve).ok_or_else(|| {
            error(
                path,
                "unknown easing; use linear, ease, ease-in, ease-out or ease-in-out",
            )
        })?;
        let owner = self.entity;
        let id = self
            .tween_store(path)?
            .insert(Track {
                owner,
                from: from.clone(),
                to: to.clone(),
                duration: *duration,
                elapsed: 0.0,
                easing,
                paused: false,
                cancelled: false,
            })
            .map_err(|message| error(path, message))?;
        Ok(Value::Reference(id))
    }

    pub(super) fn tween_call(
        &mut self,
        name: &str,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        if let Some((_, call)) = CALLS.iter().find(|(known, _)| *known == name) {
            return self.tween_create(*call, path, args);
        }
        let [Value::Reference(id)] = args else {
            return Err(error(path, "takes one valid tween handle"));
        };
        let store = self.tween_store(path)?;
        let track = store
            .get_mut(*id)
            .ok_or_else(|| error(path, "tween has been disposed or its owner removed"))?;
        if let Some(kind) = name.strip_suffix("_value") {
            let call = CALLS
                .iter()
                .find(|(known, _)| *known == kind)
                .map(|(_, call)| *call)
                .ok_or_else(|| error(path, "unknown tween value accessor"))?;
            if !call.accepts(&track.from) {
                return Err(error(path, "wrong tween value type"));
            }
            return Ok(track.value());
        }
        let value = match name {
            "progress" => Value::Number(track.progress()),
            "is_done" => Value::Bool(track.done()),
            "is_paused" => Value::Bool(track.paused),
            "is_cancelled" => Value::Bool(track.cancelled),
            name if METHODS.contains(&name) => {
                match name {
                    "pause" => track.paused = true,
                    "resume" => track.paused = false,
                    "cancel" => {
                        track.cancelled = true;
                        track.paused = false;
                    }
                    "restart" => {
                        track.elapsed = 0.0;
                        track.cancelled = false;
                        track.paused = false;
                    }
                    "dispose" => store.dispose(*id),
                    _ => unreachable!(),
                }
                Value::Unit
            }
            _ => return Err(error(path, "unknown tween control")),
        };
        Ok(value)
    }
}
