//! World-space aim and orbit, converted to the entity's stored parent space.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};

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
}
