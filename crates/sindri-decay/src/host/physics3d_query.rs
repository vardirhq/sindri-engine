//! Typed 3D queries over active synchronized geometry.

use std::rc::Rc;

use decay_ir::{Path, StructShape};
use decay_runtime::{RuntimeError, Value};
use sindri_physics::{ColliderShape3d, PhysicsPose3d, RayHit3d, RaycastFilter3d};

use super::{WorldHost, physics3d::vector};
use crate::surface::physics3d::{HIT_FIELDS, Physics3dCall, RAY_HIT};

impl WorldHost<'_> {
    pub(super) fn physics3d_query(
        &self,
        call: Physics3dCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        let filter_at = match call {
            Physics3dCall::Raycast => 3,
            Physics3dCall::OverlapSphere => 2,
            Physics3dCall::CastSphere | Physics3dCall::OverlapBox => 4,
            Physics3dCall::CastBox => 6,
            Physics3dCall::OverlapCapsule => 5,
            Physics3dCall::CastCapsule => 7,
            _ => unreachable!("only query calls reach here"),
        };
        if args.len() != filter_at + 3 {
            return Err(error("incorrect argument count"));
        }
        let position = vector(path, &args[0])?;
        // The mask/sensor/exclude decoder is dimension-independent; convert its
        // historical 2D result explicitly rather than sharing simulation types.
        let parsed = super::raycast::filter(path, args, filter_at)?;
        let filter = RaycastFilter3d {
            mask: parsed.mask,
            include_sensors: parsed.include_sensors,
            exclude: parsed.exclude,
        };
        let physics = self
            .physics3d
            .as_ref()
            .ok_or_else(|| error("host has no 3D physics"))?;
        let active = |entity| self.world.is_active(entity);
        if matches!(call, Physics3dCall::Raycast) {
            let hit = physics
                .world
                .raycast_where(
                    position,
                    vector(path, &args[1])?,
                    scalar(path, &args[2])?,
                    filter,
                    active,
                )
                .map_err(|failure| error(&failure.to_string()))?;
            return Ok(hit.map_or(Value::Null, snapshot));
        }
        let (shape, pose) = probe(call, path, args, position)?;
        if matches!(
            call,
            Physics3dCall::OverlapSphere
                | Physics3dCall::OverlapBox
                | Physics3dCall::OverlapCapsule
        ) {
            let entities = physics
                .world
                .overlap_where(shape, pose, filter, active)
                .map_err(|failure| error(&failure.to_string()))?;
            return Ok(Value::array(
                entities
                    .into_iter()
                    .map(|entity| Value::Reference(entity.to_bits()))
                    .collect(),
            ));
        }
        let hit = physics
            .world
            .shape_cast_where(
                shape,
                pose,
                vector(path, &args[filter_at - 2])?,
                scalar(path, &args[filter_at - 1])?,
                filter,
                active,
            )
            .map_err(|failure| error(&failure.to_string()))?;
        Ok(hit.map_or(Value::Null, |hit| {
            snapshot(RayHit3d {
                entity: hit.entity,
                point: hit.point,
                normal: hit.normal,
                distance: hit.distance,
            })
        }))
    }
}

fn probe(
    call: Physics3dCall,
    path: &Path,
    args: &[Value],
    position: [f32; 3],
) -> Result<(ColliderShape3d, PhysicsPose3d), RuntimeError> {
    let (shape, rotation) = match call {
        Physics3dCall::OverlapSphere | Physics3dCall::CastSphere => (
            ColliderShape3d::Sphere {
                radius: scalar(path, &args[1])?,
            },
            PhysicsPose3d::default().rotation,
        ),
        Physics3dCall::OverlapBox | Physics3dCall::CastBox => (
            ColliderShape3d::Box {
                half_extents: vector(path, &args[1])?,
            },
            rotation(path, &args[2], &args[3])?,
        ),
        Physics3dCall::OverlapCapsule | Physics3dCall::CastCapsule => (
            ColliderShape3d::Capsule {
                half_height: scalar(path, &args[1])?,
                radius: scalar(path, &args[2])?,
            },
            rotation(path, &args[3], &args[4])?,
        ),
        _ => unreachable!("only shape query calls reach here"),
    };
    Ok((shape, PhysicsPose3d { position, rotation }))
}

// Normalize in f64 so finite nonzero f32 axes, including tiny components,
// produce a finite unit quaternion without squared-length overflow/underflow.
fn rotation(path: &Path, axis: &Value, angle: &Value) -> Result<[f32; 4], RuntimeError> {
    let [x, y, z] = vector(path, axis)?.map(f64::from);
    let length = x.hypot(y).hypot(z);
    if length <= 0.0 {
        return Err(RuntimeError::Host(format!(
            "{} requires a nonzero rotation axis",
            path.dotted()
        )));
    }
    let (sin, cos) = (f64::from(scalar(path, angle)?) * 0.5).sin_cos();
    Ok([x / length * sin, y / length * sin, z / length * sin, cos].map(super::convert::as_f32))
}

fn scalar(path: &Path, value: &Value) -> Result<f32, RuntimeError> {
    let value = super::convert::number(path, value)?;
    if !value.is_finite() || value.abs() > f64::from(f32::MAX) {
        return Err(RuntimeError::Host(format!(
            "{} requires finite numbers within f32 range",
            path.dotted()
        )));
    }
    Ok(super::convert::as_f32(value))
}

fn snapshot(hit: RayHit3d) -> Value {
    Value::Struct {
        shape: Rc::new(StructShape {
            name: RAY_HIT.to_owned(),
            fields: HIT_FIELDS.into_iter().map(str::to_owned).collect(),
        }),
        fields: Rc::new(vec![
            Value::Reference(hit.entity.to_bits()),
            Value::Vec3(hit.point.map(f64::from)),
            Value::Vec3(hit.normal.map(f64::from)),
            Value::Number(f64::from(hit.distance)),
        ]),
    }
}
