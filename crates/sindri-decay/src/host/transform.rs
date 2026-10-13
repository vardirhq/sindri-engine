//! World-space aim and orbit, converted to the entity's stored parent space.

use crate::surface::Leaf;
use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::EntityId;

use super::{WorldHost, convert::as_f32, physics3d::vector};
use crate::surface::transform::{CALLS, TransformCall};

impl WorldHost<'_> {
    pub(super) fn transform_call(
        &mut self,
        subject: Option<u64>,
        path: &Path,
        args: &[Value],
    ) -> Result<Option<Value>, RuntimeError> {
        let Some(parts) = Self::addressed(subject, path) else {
            return Ok(None);
        };
        let name = match parts.as_slice() {
            ["transform", name] | ["entity", "transform", name] => *name,
            _ => return Ok(None),
        };
        let Some((_, call)) = CALLS.iter().find(|(known, _)| *known == name) else {
            return Ok(None);
        };
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        let entity = self.subject(subject, path)?;
        let local = self
            .world
            .get(entity)
            .and_then(|data| data.transform_3d)
            .ok_or_else(|| error("entity has no transform"))?;
        let parent = self.world.parent_space(entity);
        if matches!(call, TransformCall::RotateAround)
            && parent.scale.iter().any(|v| v.abs() <= f32::EPSILON)
        {
            return Err(error("cannot orbit under a parent with zero scale"));
        }
        let mut placed = self
            .world
            .world_transform(entity)
            .ok_or_else(|| error("entity has no transform"))?;
        match (call, args) {
            (TransformCall::LookAt, [target]) => placed.look_at(vector(path, target)?),
            (TransformCall::RotateAround, [pivot, axis, Value::Number(angle)]) => {
                if !angle.is_finite() || angle.abs() > f64::from(f32::MAX) {
                    return Err(error("angle must be finite and within f32 range"));
                }
                placed.rotate_around(vector(path, pivot)?, vector(path, axis)?, as_f32(*angle))
            }
            _ => return Err(error("incorrect transform arguments")),
        }
        .map_err(|cause| error(&cause.to_string()))?;
        // A zero turn is a true no-op, including under a scaled parent:
        // converting an unchanged world pose back could round its local position.
        if matches!((call, args), (TransformCall::RotateAround, [_, _, Value::Number(angle)]) if angle.abs() <= 0.0)
        {
            return Ok(Some(Value::Unit));
        }
        let mut next = self.world.local_for_world(entity, placed);
        // Aiming must not round-trip translation through a scaled parent.
        if matches!(call, TransformCall::LookAt) {
            next.position = local.position;
        }
        next.scale = local.scale;
        if !next
            .position
            .into_iter()
            .chain(next.rotation)
            .all(f32::is_finite)
        {
            return Err(error("operation produced a non-finite local transform"));
        }
        if local.z_lock_rejects(Some(next)) {
            return Err(error(
                "operation would move a Z-locked transform off its layer",
            ));
        }
        self.world
            .get_mut(entity)
            .ok_or_else(|| error("entity no longer exists"))?
            .transform_3d = Some(next);
        Ok(Some(Value::Unit))
    }
    /// Store one transform scalar through the shared leaf description.
    pub(super) fn store_transform_leaf(
        &mut self,
        entity: EntityId,
        leaf: Leaf,
        number: f64,
        path: &Path,
    ) -> Result<bool, RuntimeError> {
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
            Leaf::Component { .. } | Leaf::TransformRotation(_) => {
                unreachable!("matched above")
            }
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
}
