//! `Physics.overlap_*` and `Physics.cast_*`: the raycast's arguments with a
//! size, answered from the same synchronized geometry.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_physics::{ColliderShape2d, PhysicsPose2d, RayHit2d};

use super::raycast::{filter, snapshot, vector};
use super::{
    WorldHost,
    convert::{as_f32, number},
};
use crate::surface::PhysicsCall;

impl WorldHost<'_> {
    pub(super) fn physics_shape_query(
        &self,
        call: PhysicsCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        let scalar = |at: usize| -> Result<f32, RuntimeError> {
            Ok(as_f32(number(path, args.get(at).unwrap_or(&Value::Null))?))
        };
        let position = vector(path, args.first())?;
        // A circle is one number after its centre; a box is its half size and
        // a rotation. Whatever follows starts after that.
        let (shape, rotation, rest) = match call {
            PhysicsCall::OverlapCircle | PhysicsCall::CastCircle => {
                (ColliderShape2d::Circle { radius: scalar(1)? }, 0.0, 2)
            }
            _ => (
                ColliderShape2d::Box {
                    half_extents: vector(path, args.get(1))?,
                },
                scalar(2)?,
                3,
            ),
        };
        let pose = PhysicsPose2d { position, rotation };
        let Some(physics) = self.physics.as_ref() else {
            return Err(error("needs physics, and this host is not running any"));
        };
        let active = |entity| self.world.is_active(entity);
        if matches!(call, PhysicsCall::OverlapCircle | PhysicsCall::OverlapBox) {
            let filter = filter(path, args, rest)?;
            let found = physics
                .world
                .overlap_where(shape, pose, filter, active)
                .map_err(|failure| error(&failure.to_string()))?;
            return Ok(Value::array(
                found
                    .into_iter()
                    .map(|entity| Value::Reference(entity.to_bits()))
                    .collect(),
            ));
        }
        let direction = vector(path, args.get(rest))?;
        let distance = scalar(rest + 1)?;
        let filter = filter(path, args, rest + 2)?;
        let hit = physics
            .world
            .shape_cast_where(shape, pose, direction, distance, filter, active)
            .map_err(|failure| error(&failure.to_string()))?;
        Ok(hit.map_or(Value::Null, |hit| {
            snapshot(RayHit2d {
                entity: hit.entity,
                point: hit.point,
                normal: hit.normal,
                distance: hit.distance,
            })
        }))
    }
}
