//! Live 3D controls and shared last-step event observation.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_physics::PhysicsEventKind;

use super::WorldHost;
use crate::surface::physics3d::Physics3dCall;

impl WorldHost<'_> {
    pub(super) fn physics3d_call(
        &mut self,
        call: Physics3dCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        if matches!(call, Physics3dCall::Layer | Physics3dCall::Mask) {
            return self.physics3d_layers(call, path, args);
        }
        if matches!(
            call,
            Physics3dCall::Raycast | Physics3dCall::OverlapSphere | Physics3dCall::CastSphere
        ) {
            return self.physics3d_query(call, path, args);
        }
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        let wanted = match call {
            Physics3dCall::CollisionStarted => Some(PhysicsEventKind::CollisionStarted),
            Physics3dCall::CollisionStopped => Some(PhysicsEventKind::CollisionStopped),
            Physics3dCall::SensorEntered => Some(PhysicsEventKind::SensorEntered),
            Physics3dCall::SensorExited => Some(PhysicsEventKind::SensorExited),
            _ => None,
        };
        if let Some(wanted) = wanted {
            return self.physics3d_events(wanted, path, args);
        }
        let read = matches!(
            call,
            Physics3dCall::Velocity | Physics3dCall::AngularVelocity
        );
        if args.len() != if read { 1 } else { 2 } {
            return Err(error("incorrect argument count"));
        }
        let entity = self.entity_argument(path, args, 0, "the 3D body")?;
        if !self.world.is_active(entity) {
            return Err(error("3D body must be active"));
        }
        let vector = if read {
            [0.0; 3]
        } else {
            vector(path, &args[1])?
        };
        let physics = self
            .physics3d
            .as_mut()
            .ok_or_else(|| error("host has no 3D physics"))?;
        match call {
            Physics3dCall::Velocity | Physics3dCall::AngularVelocity => {
                let value = if matches!(call, Physics3dCall::Velocity) {
                    physics.world.linear_velocity(entity)
                } else {
                    physics.world.angular_velocity(entity)
                }
                .map_err(|failure| error(&failure.to_string()))?;
                Ok(Value::Vec3(value.map(f64::from)))
            }
            _ => {
                let result = match call {
                    Physics3dCall::SetVelocity => physics.world.set_linear_velocity(entity, vector),
                    Physics3dCall::SetAngularVelocity => {
                        physics.world.set_angular_velocity(entity, vector)
                    }
                    Physics3dCall::ApplyImpulse => physics.world.apply_impulse(entity, vector),
                    _ => unreachable!("event and read calls handled above"),
                };
                result.map_err(|failure| error(&failure.to_string()))?;
                Ok(Value::Unit)
            }
        }
    }
    fn physics3d_events(
        &self,
        wanted: PhysicsEventKind,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        if !args.is_empty() {
            return Err(error("event queries take no arguments"));
        }
        let physics = self
            .physics3d
            .as_ref()
            .ok_or_else(|| error("host has no 3D physics"))?;
        let mut others: Vec<_> = physics
            .events
            .iter()
            .filter(|event| event.kind == wanted)
            .filter_map(|event| {
                if event.first == self.entity {
                    Some(event.second)
                } else if event.second == self.entity {
                    Some(event.first)
                } else {
                    None
                }
            })
            .filter(|other| self.world.is_active(*other))
            .collect();
        others.sort_unstable();
        others.dedup();
        Ok(Value::array(
            others
                .into_iter()
                .map(|other| Value::Reference(other.to_bits()))
                .collect(),
        ))
    }
}

pub(super) fn vector(path: &Path, value: &Value) -> Result<[f32; 3], RuntimeError> {
    let Value::Vec3(parts) = value else {
        return Err(RuntimeError::Host(format!(
            "{} requires a Vec3",
            path.dotted()
        )));
    };
    if !parts
        .iter()
        .all(|part| part.is_finite() && part.abs() <= f64::from(f32::MAX))
    {
        return Err(RuntimeError::Host(format!(
            "{} requires finite Vec3 components within f32 range",
            path.dotted()
        )));
    }
    Ok(parts.map(super::convert::as_f32))
}
