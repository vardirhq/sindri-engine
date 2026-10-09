//! The runtime host boundary: loading, storing and calling script paths.
//!
//! Entity addressing and vector access share the same leaf lookup as scalar
//! access. Keeping them with the `Host` implementation makes that boundary
//! independent of the services and context the parent module owns.

use decay_ir::Path;
use decay_runtime::{Host, RuntimeError, Value};
use sindri_core::{EntityId, Transform3D};

use super::convert::{as_f32, describe, number};
use super::{WorldHost, print};
use crate::surface::{
    AIM, AIM_VALUES, CAMERA, CAMERA_VALUES, FUNCTIONS, GESTURE, GESTURE_VALUES, Handle,
    HostFunction, Leaf, POINTER, POINTER_VALUES, PRINT, STICK, STICK_VALUES, TOUCH, TOUCH_COUNT,
    VIEWPORT, VIEWPORT_VALUES, ViewportValue, follow_mut, handle, leaf, leaf_through_reference,
    ungrouped, vector_components,
};

impl Host for WorldHost<'_> {
    fn load(&mut self, subject: Option<u64>, path: &Path) -> Result<Option<Value>, RuntimeError> {
        let parts: Vec<&str> = path.0.iter().map(String::as_str).collect();

        // `Game.score`: a declared state, before any namespace the engine
        // offers, since a state may add to one.
        if subject.is_none()
            && let Some(value) = self.shared_load(path)
        {
            return Ok(Some(value));
        }

        if subject.is_none()
            && let Some(value) = self.time_or_constant(&parts)
        {
            return Ok(Some(value));
        }

        // Where the person is pointing, and how many fingers are down. Facts
        // about the frame like `Time.delta`, and never about a subject: a
        // reference cannot be asked where the mouse is.
        if subject.is_none()
            && let [namespace, name] = ungrouped(&parts)
        {
            if *namespace == POINTER
                && let Some((_, value)) = POINTER_VALUES.iter().find(|(known, _)| known == name)
            {
                return Ok(Some(self.pointer_value(*value)));
            }
            if *namespace == STICK
                && let Some((_, value)) = STICK_VALUES.iter().find(|(known, _)| known == name)
            {
                return Ok(Some(self.stick_value(*value)));
            }
            if *namespace == TOUCH && *name == TOUCH_COUNT {
                // `usize` to `f64` is exact for every count a hand can produce,
                // and the platform bounds it to ten regardless.
                #[allow(clippy::cast_precision_loss)]
                return Ok(Some(Value::Number(self.context.input.touch_count() as f64)));
            }
            if *namespace == CAMERA
                && let Some((_, value)) = CAMERA_VALUES.iter().find(|(known, _)| known == name)
            {
                return Ok(Some(self.camera_value(*value)));
            }
            if *namespace == GESTURE
                && let Some((_, value)) = GESTURE_VALUES.iter().find(|(known, _)| known == name)
            {
                return Ok(Some(self.gesture_value(*value)));
            }
            if *namespace == AIM
                && let Some((_, value)) = AIM_VALUES.iter().find(|(known, _)| known == name)
            {
                return Ok(Some(self.aim_value(*value)));
            }
            if *namespace == VIEWPORT
                && let Some((_, value)) = VIEWPORT_VALUES.iter().find(|(known, _)| known == name)
            {
                return Ok(Some(Value::Number(f64::from(match value {
                    ViewportValue::Aspect => self
                        .screen_ui
                        .map_or(1.0, sindri_scene::ScreenUi::viewport_aspect),
                }))));
            }
        }

        let Some(under) = Self::addressed(subject, path) else {
            return Ok(None);
        };

        // A reference is fetched rather than read into, so it is answered
        // before any leaf lookup: `this.entity` is not a number.
        if subject.is_none()
            && let Some(handle) = handle(&under)
        {
            return Ok(Some(match handle {
                Handle::Own => Value::Reference(self.entity.to_bits()),
            }));
        }

        let found = if subject.is_some() {
            leaf_through_reference(&under)
        } else {
            leaf(&under)
        };
        let Some(leaf) = found else {
            if let Some(vector) = self.load_vector(subject, path, &under)? {
                return Ok(Some(vector));
            }
            return self.peer_load(subject, path);
        };
        let entity = self.subject(subject, path)?;
        let transform = self.transform_of(entity);
        // Read in place. Every load used to serialize all of the entity's
        // components into a fresh JSON object to look one number up, which was
        // most of what Orbital's scripts spent their time on.
        let components = self.world.get(entity).map(|data| &data.components);

        // `None` is how the runtime says "unknown path" with the name attached,
        // and it is the right answer for an entity that has no sprite: the
        // surface says a script *may* reach one, not that every entity has one.
        Ok(leaf
            .read(
                transform.as_ref(),
                || self.world.world_transform(entity),
                components,
            )
            .map(Value::Number))
    }

    fn store(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        value: Value,
    ) -> Result<bool, RuntimeError> {
        if subject.is_none() && self.shared_store(path, &value)? {
            return Ok(true);
        }
        let Some(under) = Self::addressed(subject, path) else {
            return Ok(false);
        };
        let found = if subject.is_some() {
            leaf_through_reference(&under)
        } else {
            leaf(&under)
        };
        let Some(leaf) = found else {
            if self.store_vector(subject, path, &under, &value)? {
                return Ok(true);
            }
            return self.peer_store(subject, path, value);
        };
        if leaf.is_read_only() {
            return Err(RuntimeError::Host(format!(
                "{} is read-only",
                path.dotted()
            )));
        }
        let entity = self.subject(subject, path)?;
        let number = number(path, &value)?;

        match leaf {
            Leaf::TransformAxis(..) | Leaf::TransformScalar(_) => {
                let Some(mut transform) = self.transform_of(entity) else {
                    return Ok(false);
                };
                match leaf {
                    Leaf::TransformAxis(vector, index) if vector.is_world() => {
                        // Moved in the world, and stored as the place under its
                        // parent that puts it there.
                        let Some(mut placed) = self.world.world_transform(entity) else {
                            return Ok(false);
                        };
                        let mut values = vector.get(&placed);
                        values[index] = as_f32(number);
                        vector.set(&mut placed, values);
                        transform = self.world.local_for_world(entity, placed);
                    }
                    Leaf::TransformAxis(vector, index) => {
                        let mut values = vector.get(&transform);
                        values[index] = as_f32(number);
                        vector.set(&mut transform, values);
                    }
                    Leaf::TransformScalar(scalar) => {
                        scalar.set(&mut transform, as_f32(number));
                    }
                    Leaf::Component { .. } => unreachable!("matched above"),
                }

                // The Z lock is an invariant of the transform, not a rule the
                // editor enforces on the way in. A script is a write path like
                // any other, and one that could ignore the lock would be the
                // hole that makes the lock worthless.
                if self
                    .transform_of(entity)
                    .is_some_and(|current| current.z_lock_rejects(Some(transform)))
                {
                    return Err(RuntimeError::Host(format!(
                        "{} would move a Z-locked transform off its layer",
                        path.dotted()
                    )));
                }
                let Some(data) = self.world.get_mut(entity) else {
                    return Ok(false);
                };
                data.transform_3d = Some(transform);
                Ok(true)
            }
            Leaf::Component { component, pointer } => {
                let Some(data) = self.world.get_mut(entity) else {
                    return Ok(false);
                };
                let Some(payload) = data.components.get_mut(component) else {
                    return Err(RuntimeError::Host(format!(
                        "{} needs a {component} on this entity, and it has none",
                        path.dotted()
                    )));
                };
                let Some(slot) = follow_mut(payload, pointer) else {
                    return Err(RuntimeError::Host(format!(
                        "{}'s {component} has nothing at {}",
                        path.dotted(),
                        describe(pointer)
                    )));
                };
                // Written as the number the payload already held it as, so a
                // layer stays an integer and a tint channel stays a float --
                // the scene round-trips byte for byte either way.
                *slot = if slot.is_i64() || slot.is_u64() {
                    // Rounded and narrowed on purpose: the payload held an
                    // integer, and a layer that came back as `7.0` would change
                    // a scene byte for byte because a script touched it. A
                    // number too large for an i64 saturates, which is a wrong
                    // layer rather than a wrong file.
                    #[allow(clippy::cast_possible_truncation)]
                    serde_json::Value::from(number.round() as i64)
                } else {
                    serde_json::Value::from(number)
                };
                Ok(true)
            }
        }
    }

    fn call(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        args: &[Value],
    ) -> Result<Option<Value>, RuntimeError> {
        if let Some(value) = self.transform_call(subject, path, args)? {
            return Ok(Some(value));
        }
        // Transform methods above are the entity methods. Other reference calls
        // address script messages; refusing namespace calls here keeps
        // `target.axis("a", "b")` from reaching `Input`.
        if subject.is_some() {
            return Ok(self.peer_call(subject, path, args));
        }
        let parts: Vec<&str> = path.0.iter().map(String::as_str).collect();

        if let Some(value) = self.bare_call(&parts, path, args)? {
            return Ok(Some(value));
        }

        if let [namespace, name] = ungrouped(&parts)
            && let Some(result) = self.namespaced_call(namespace, name, path, args)
        {
            return result.map(Some);
        }

        // `Bolt.on(entity)`: a script found by its type.
        if let [script, name] = parts.as_slice()
            && *name == crate::scripts::ON
        {
            return self.script_on(script, path, args);
        }

        if let [event, name] = parts.as_slice()
            && *name == decay_semantic::EMIT
            && let Some(value) = self.emit(event, args)
        {
            return Ok(Some(value));
        }

        Ok(None)
    }
}

impl WorldHost<'_> {
    /// Everything the script printed during the call.
    pub fn take_printed(&mut self) -> Vec<String> {
        std::mem::take(&mut self.printed)
    }

    /// `print` and the maths, which are the only calls with no namespace.
    ///
    /// Their own function because the dispatch that follows them is one arm per
    /// namespace, and a reader looking for `World.spawn` should not have to
    /// scroll past the argument handling for `min`.
    fn bare_call(
        &mut self,
        parts: &[&str],
        path: &Path,
        args: &[Value],
    ) -> Result<Option<Value>, RuntimeError> {
        let [name] = parts else {
            return Ok(None);
        };
        {
            if *name == PRINT {
                self.printed.push(print::printed(args.first()));
                return Ok(Some(Value::Unit));
            }
            if let Some((_, function)) = FUNCTIONS.iter().find(|(known, _)| known == name) {
                let argument = |index: usize| -> Result<f64, RuntimeError> {
                    args.get(index)
                        .ok_or_else(|| {
                            RuntimeError::Host(format!("{} wants more arguments", path.dotted()))
                        })
                        .and_then(|value| number(path, value))
                };
                return Ok(Some(Value::Number(match function {
                    HostFunction::Unary(apply) => apply(argument(0)?),
                    HostFunction::Binary(apply) => apply(argument(0)?, argument(1)?),
                    HostFunction::Ternary(apply) => apply(argument(0)?, argument(1)?, argument(2)?),
                })));
            }
        }
        Ok(None)
    }

    /// One of the numbers a script reads about the pointer.
    ///
    /// A pointer that is not there reads as zero rather than as an error,
    /// because "the mouse left the window" is an ordinary thing that happens
    /// mid-frame and not a mistake in a script. `Pointer.inside` is how a
    /// script that cares asks, and it has to be asked *before* the position is
    /// believed.
    pub(super) fn transform_of(&self, entity: EntityId) -> Option<Transform3D> {
        self.world.get(entity)?.transform_3d
    }

    /// Which entity a call is about: the one the script runs on, or the one a
    /// reference names.
    ///
    /// A reference that no longer resolves is an error naming the path rather
    /// than a silent no-op, because a script holding a stale handle is a bug in
    /// the script and the whole point of generation checking is to say so.
    pub(super) fn subject(
        &self,
        subject: Option<u64>,
        path: &Path,
    ) -> Result<EntityId, RuntimeError> {
        let Some(bits) = subject else {
            return Ok(self.entity);
        };
        let entity = EntityId::from_bits(bits);
        if self.world.get(entity).is_some() {
            Ok(entity)
        } else {
            Err(RuntimeError::Host(format!(
                "{} is about an entity that no longer exists",
                path.dotted()
            )))
        }
    }

    /// The parts of a path that address an entity's members.
    ///
    /// With a subject the path is already rooted at it, so every part counts.
    /// Without one the path is the script's own and starts with `this`.
    /// A whole vector, read one component at a time through the leaves that
    /// answer each of them.
    fn load_vector(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        under: &[&str],
    ) -> Result<Option<Value>, RuntimeError> {
        let Some(components) = vector_components(under, subject.is_some()) else {
            return Ok(None);
        };
        let mut numbers = Vec::with_capacity(components.len());
        for (name, _) in components {
            let mut part = path.clone();
            part.0.push((*name).to_owned());
            match self.load(subject, &part)? {
                Some(Value::Number(number)) => numbers.push(number),
                _ => return Ok(None),
            }
        }
        Ok(Value::vector(&numbers))
    }

    /// A whole vector, written one component at a time.
    fn store_vector(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        under: &[&str],
        value: &Value,
    ) -> Result<bool, RuntimeError> {
        let Some(components) = vector_components(under, subject.is_some()) else {
            return Ok(false);
        };
        let numbers = value
            .components()
            .filter(|numbers| numbers.len() == components.len())
            .ok_or_else(|| {
                RuntimeError::Host(format!(
                    "{} takes a {}, and the script gave {value:?}",
                    path.dotted(),
                    if components.len() == 4 {
                        "Color".to_owned()
                    } else {
                        format!("Vec{}", components.len())
                    }
                ))
            })?
            .to_vec();
        for ((name, _), number) in components.iter().zip(numbers) {
            let mut part = path.clone();
            part.0.push((*name).to_owned());
            if !self.store(subject, &part, Value::Number(number))? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub(super) fn addressed(subject: Option<u64>, path: &Path) -> Option<Vec<&str>> {
        if subject.is_some() {
            return Some(path.0.iter().map(String::as_str).collect());
        }
        Self::under_this(path)
    }

    /// The parts of a path after `this`, when it starts with `this`.
    pub(super) fn under_this(path: &Path) -> Option<Vec<&str>> {
        let mut parts = path.0.iter().map(String::as_str);
        (parts.next()? == "this").then(|| parts.collect())
    }
}
