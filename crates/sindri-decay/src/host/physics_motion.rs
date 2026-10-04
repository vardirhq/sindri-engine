//! Typed world-space forces and rotation controls.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_physics::{BodyControl2d, RigidBody2d};

use super::{
    WorldHost,
    convert::{as_f32, number},
    raycast::vector,
};
use crate::surface::PhysicsCall;

impl WorldHost<'_> {
    pub(super) fn physics_motion_call(
        &mut self,
        call: PhysicsCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let entity = self.entity_argument(path, args, 0, "the body")?;
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        let authored: RigidBody2d = self
            .world
            .get(entity)
            .and_then(|data| data.components.get("sindri.physics2d.rigid_body"))
            .ok_or_else(|| error("entity has no authored 2D rigid body"))
            .and_then(|payload| {
                serde_json::from_value(payload.clone())
                    .map_err(|failure| error(&format!("invalid rigid body: {failure}")))
            })?;
        let scalar = || number(path, args.get(1).unwrap_or(&Value::Null)).map(as_f32);
        let control = match call {
            PhysicsCall::ApplyForce => BodyControl2d::Force(vector(path, args.get(1))?),
            PhysicsCall::ApplyTorque => BodyControl2d::Torque(scalar()?),
            PhysicsCall::SetAngularVelocity => BodyControl2d::AngularVelocity({
                let velocity = scalar()?;
                if !velocity.is_finite() {
                    return Err(error("angular velocity must be finite"));
                }
                if authored.lock_rotation {
                    0.0
                } else {
                    velocity
                }
            }),
            PhysicsCall::ApplyAngularImpulse => BodyControl2d::AngularImpulse(scalar()?),
            PhysicsCall::ApplyImpulseAtPoint => BodyControl2d::ImpulseAtPoint {
                impulse: vector(path, args.get(1))?,
                point: vector(path, args.get(2))?,
            },
            PhysicsCall::ApplyImpulse => BodyControl2d::Impulse([
                as_f32(number(path, args.get(1).unwrap_or(&Value::Null))?),
                as_f32(number(path, args.get(2).unwrap_or(&Value::Null))?),
            ]),
            PhysicsCall::AngularVelocity => {
                let Some(physics) = self.physics.as_ref() else {
                    return Err(error("needs physics, and this host is not running any"));
                };
                let velocity = match physics.world.angular_velocity(entity) {
                    Ok(value) => value,
                    Err(sindri_physics::PhysicsError::MissingEntity(_))
                        if !physics.world.contains(entity) =>
                    {
                        if authored.lock_rotation {
                            0.0
                        } else {
                            authored.angular_velocity
                        }
                    }
                    Err(failure) => return Err(error(&failure.to_string())),
                };
                return Ok(Value::Number(f64::from(velocity)));
            }
            _ => unreachable!("only motion controls reach here"),
        };
        let Some(physics) = self.physics.as_mut() else {
            return Err(error("needs physics, and this host is not running any"));
        };
        let outcome = if physics.world.contains(entity) {
            physics.world.apply_control(entity, control)
        } else {
            physics
                .world
                .remember_control(entity, authored.kind, control)
        };
        outcome.map_err(|failure| error(&failure.to_string()))?;
        Ok(Value::Unit)
    }
}
