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
            Physics3dCall::Raycast
                | Physics3dCall::OverlapSphere
                | Physics3dCall::CastSphere
                | Physics3dCall::OverlapBox
                | Physics3dCall::CastBox
                | Physics3dCall::OverlapCapsule
                | Physics3dCall::CastCapsule
        ) {
            return self.physics3d_query(call, path, args);
        }
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
        self.physics3d_motion(call, path, args)
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
