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
        if matches!(call, TransformCall::ToLocal | TransformCall::ToWorld) {
            let placed = self
                .world
                .world_transform(entity)
                .ok_or_else(|| error("entity has no transform"))?;
            let [point] = args else {
                return Err(error("takes one Vec3 point"));
            };
            let point = vector(path, point)?;
            let out = if matches!(call, TransformCall::ToWorld) {
                let scaled = [0, 1, 2].map(|axis| point[axis] * placed.scale[axis]);
                let turned = rotate(placed.rotation, scaled);
                [0, 1, 2].map(|axis| placed.position[axis] + turned[axis])
            } else {
                if placed.scale.iter().any(|axis| axis.abs() <= f32::EPSILON) {
                    return Err(error("cannot measure from an entity with zero scale"));
                }
                let [x, y, z, w] = placed.rotation;
                let offset = [0, 1, 2].map(|axis| point[axis] - placed.position[axis]);
                let turned = rotate([-x, -y, -z, w], offset);
                [0, 1, 2].map(|axis| turned[axis] / placed.scale[axis])
            };
            if !out.iter().all(|axis| axis.is_finite()) {
                return Err(error("the point is out of range"));
            }
            return Ok(Some(Value::Vec3(out.map(f64::from))));
        }
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
            Leaf::Component { .. }
            | Leaf::TransformRotation(_)
            | Leaf::Light(_)
            | Leaf::Environment(_) => {
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

/// A vector turned by a quaternion in `[x, y, z, w]` order, normalized first.
// x, y, z, w and t are the quaternion formula's own names.
#[allow(clippy::many_single_char_names)]
fn rotate(rotation: [f32; 4], vector: [f32; 3]) -> [f32; 3] {
    let length = rotation.iter().map(|part| part * part).sum::<f32>().sqrt();
    let [x, y, z, w] = if length > f32::EPSILON {
        rotation.map(|part| part / length)
    } else {
        [0.0, 0.0, 0.0, 1.0]
    };
    let [vx, vy, vz] = vector;
    // t = 2 (q.xyz x v); v' = v + w t + q.xyz x t
    let t = [
        2.0 * (y * vz - z * vy),
        2.0 * (z * vx - x * vz),
        2.0 * (x * vy - y * vx),
    ];
    [
        vx + w * t[0] + (y * t[2] - z * t[1]),
        vy + w * t[1] + (z * t[0] - x * t[2]),
        vz + w * t[2] + (x * t[1] - y * t[0]),
    ]
}
