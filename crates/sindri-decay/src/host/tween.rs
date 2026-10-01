//! Managed values; scripts apply them through ordinary checked write paths.
use super::WorldHost;
use crate::{
    surface::tween::{CALLS, COMPOSE, METHODS, TweenCall},
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
            .insert(Track::new(
                owner,
                from.clone(),
                to.clone(),
                *duration,
                easing,
            ))
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
        if COMPOSE.contains(&name) {
            return self.tween_compose(name, path, args);
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
                        track.waited = 0.0;
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

    /// `set_delay`, `set_loops`, `set_yoyo` and `after`: how a tween plays,
    /// set on its handle, usually as it is made.
    fn tween_compose(
        &mut self,
        name: &str,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let Some(Value::Reference(id)) = args.first() else {
            return Err(error(path, "takes a valid tween handle first"));
        };
        let id = *id;
        let store = self.tween_store(path)?;
        // Checked before the track is borrowed: a sequence names a second one.
        let before = match (name, args.get(1)) {
            ("after", Some(Value::Reference(before))) => {
                if *before == id {
                    return Err(error(path, "a tween cannot wait for itself"));
                }
                if store.get_mut(*before).is_none() {
                    return Err(error(path, "the tween to wait for has been disposed"));
                }
                Some(*before)
            }
            ("after", _) => return Err(error(path, "takes the tween to wait for")),
            _ => None,
        };
        let track = store
            .get_mut(id)
            .ok_or_else(|| error(path, "tween has been disposed or its owner removed"))?;
        match (name, args.get(1)) {
            ("after", _) => track.after = before,
            ("set_delay", Some(Value::Number(seconds)))
                if seconds.is_finite() && *seconds >= 0.0 =>
            {
                track.delay = *seconds;
            }
            ("set_loops", Some(Value::Number(count)))
                if count.is_finite() && *count >= 0.0 && count.fract() == 0.0 =>
            {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let count = count.min(f64::from(u32::MAX)) as u32;
                track.loops = count;
            }
            ("set_yoyo", Some(Value::Bool(on))) => track.yoyo = *on,
            ("set_delay", _) => {
                return Err(error(path, "delay must be finite, non-negative seconds"));
            }
            ("set_loops", _) => {
                return Err(error(
                    path,
                    "loops must be a whole number; 0 plays for ever",
                ));
            }
            _ => return Err(error(path, "takes a tween handle and a true/false")),
        }
        Ok(Value::Unit)
    }
}
