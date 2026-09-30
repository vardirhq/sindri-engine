//! Deterministic snapshots over active, authored tags.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::{EntityId, SceneComponent, TagsComponent};

use super::WorldHost;

/// Result bound shared by tag and radius snapshots. Never silently truncate.
const QUERY_LIMIT: usize = 8192;

impl WorldHost<'_> {
    pub(super) fn with_tag_call(&self, path: &Path, args: &[Value]) -> Result<Value, RuntimeError> {
        let tag = tag_argument(path, args)?;
        let mut found = Vec::new();
        for (entity, _) in self.world.entities() {
            if self.active_tagged(entity, tag, path)? {
                check_limit(path, tag, found.len())?;
                found.push(Value::Reference(entity.to_bits()));
            }
        }
        Ok(Value::array(found))
    }

    pub(super) fn has_tag_call(&self, path: &Path, args: &[Value]) -> Result<Value, RuntimeError> {
        let entity = self.entity_argument(path, args, 0, "the entity to inspect")?;
        let Some(Value::String(tag)) = args.get(1) else {
            return Err(RuntimeError::Host(format!("{} names a tag, as text", path.dotted())));
        };
        Ok(Value::Bool(self.active_tagged(entity, tag, path)?))
    }

    pub(super) fn nearest_call(&self, path: &Path, args: &[Value]) -> Result<Value, RuntimeError> {
        let (tag, position) = spatial_arguments(path, args, 2)?;
        let mut best: Option<(EntityId, f64)> = None;
        for (entity, _) in self.world.entities() {
            let Some(distance) = self.tagged_distance(entity, tag, position, path)? else {
                continue;
            };
            // Strictly less: equal distances keep the first in stable world order.
            if best.is_none_or(|(_, previous)| distance < previous) {
                best = Some((entity, distance));
            }
        }
        Ok(best.map_or(Value::Null, |(entity, _)| Value::Reference(entity.to_bits())))
    }

    pub(super) fn within_radius_call(&self, path: &Path, args: &[Value]) -> Result<Value, RuntimeError> {
        let (tag, position) = spatial_arguments(path, args, 3)?;
        let Some(Value::Number(radius)) = args.get(2) else {
            return Err(RuntimeError::Host(format!("{} takes a numeric radius", path.dotted())));
        };
        if radius.is_nan() || *radius < 0.0 {
            return Err(RuntimeError::Host(format!("{} takes a non-negative radius, not NaN", path.dotted())));
        }
        // Positive infinity deliberately means unbounded. Squaring in f64 also
        // keeps large finite radii from narrowing to an engine f32 infinity.
        let radius_squared = radius * radius;
        let mut found = Vec::new();
        for (entity, _) in self.world.entities() {
            let Some(distance) = self.tagged_distance(entity, tag, position, path)? else {
                continue;
            };
            if distance <= radius_squared {
                check_limit(path, tag, found.len())?;
                found.push((entity, distance));
            }
        }
        // Stable sort preserves world order for ties; no hash iteration or
        // handle numbering is used as a substitute for that order.
        found.sort_by(|(_, a), (_, b)| a.total_cmp(b));
        Ok(Value::array(found.into_iter().map(|(entity, _)| Value::Reference(entity.to_bits())).collect()))
    }

    fn active_tagged(&self, entity: EntityId, tag: &str, path: &Path) -> Result<bool, RuntimeError> {
        if !self.world.is_active(entity) {
            return Ok(false);
        }
        let Some(payload) = self.world.get(entity).and_then(|data| data.components.get(TagsComponent::TYPE_NAME)) else {
            return Ok(false);
        };
        let tags: TagsComponent = serde_json::from_value(payload.clone()).map_err(|error| {
            RuntimeError::Host(format!("{}: an entity's {} could not be read: {error}", path.dotted(), TagsComponent::TYPE_NAME))
        })?;
        Ok(tags.has(tag))
    }

    fn tagged_distance(&self, entity: EntityId, tag: &str, position: [f64; 3], path: &Path) -> Result<Option<f64>, RuntimeError> {
        if !self.active_tagged(entity, tag, path)? {
            return Ok(None);
        }
        let Some(transform) = self.world.world_transform(entity) else {
            return Ok(None);
        };
        if !transform.position.iter().all(|part| part.is_finite()) {
            return Ok(None);
        }
        // World positions are f32, but arithmetic here stays f64 so squaring
        // large engine coordinates cannot overflow or change query ordering.
        Ok(Some(transform.position.into_iter().zip(position).map(|(a, b)| {
            let delta = f64::from(a) - b;
            delta * delta
        }).sum()))
    }
}

fn tag_argument<'a>(path: &Path, args: &'a [Value]) -> Result<&'a str, RuntimeError> {
    let Some(Value::String(tag)) = args.first() else {
        return Err(RuntimeError::Host(format!("{} names a tag, as text", path.dotted())));
    };
    Ok(tag)
}

fn spatial_arguments<'a>(path: &Path, args: &'a [Value], arity: usize) -> Result<(&'a str, [f64; 3]), RuntimeError> {
    if args.len() != arity {
        return Err(RuntimeError::Host(format!("{} takes exactly {arity} arguments", path.dotted())));
    }
    let tag = tag_argument(path, args)?;
    let Some(Value::Vec3(position)) = args.get(1) else {
        return Err(RuntimeError::Host(format!("{} takes a world-space Vec3 position", path.dotted())));
    };
    if !position.iter().all(|part| part.is_finite() && part.abs() <= f64::from(f32::MAX)) {
        return Err(RuntimeError::Host(format!("{} takes finite position components within the engine's f32 range", path.dotted())));
    }
    Ok((tag, *position))
}

fn check_limit(path: &Path, tag: &str, count: usize) -> Result<(), RuntimeError> {
    if count == QUERY_LIMIT {
        return Err(RuntimeError::Host(format!("{} found more than {QUERY_LIMIT} entities tagged '{tag}'", path.dotted())));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
