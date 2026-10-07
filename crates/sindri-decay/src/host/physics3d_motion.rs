//! Live controls and validated authored bodies awaiting synchronization.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::{EntityId, SceneComponent};
use sindri_physics::{BodyControl3d, PhysicsWorld3d, RigidBody3d, RigidBodyKind};
use sindri_scene::{
    Character2dComponent, Collider2dComponent, Collider3dComponent, RigidBody2dComponent,
    RigidBody3dComponent, TilemapCollider2dComponent,
};

use super::{WorldHost, physics3d::vector};
use crate::surface::physics3d::Physics3dCall;

impl WorldHost<'_> {
    pub(super) fn physics3d_motion(
        &mut self,
        call: Physics3dCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
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
        let value = if read {
            [0.0; 3]
        } else {
            vector(path, &args[1])?
        };
        let live = self
            .physics3d
            .as_ref()
            .ok_or_else(|| error("host has no 3D physics"))?
            .world
            .contains(entity);
        let authored = if live {
            None
        } else {
            Some(self.pending_body3d(entity, path)?)
        };
        let physics = self
            .physics3d
            .as_mut()
            .ok_or_else(|| error("host has no 3D physics"))?;
        if read {
            let value = if let Some(body) = authored {
                if matches!(call, Physics3dCall::Velocity) {
                    physics
                        .world
                        .pending_linear_velocity(entity)
                        .unwrap_or(body.linear_velocity)
                } else if body.lock_rotation {
                    [0.0; 3]
                } else {
                    physics
                        .world
                        .pending_angular_velocity(entity)
                        .unwrap_or(body.angular_velocity)
                }
            } else if matches!(call, Physics3dCall::Velocity) {
                physics
                    .world
                    .linear_velocity(entity)
                    .map_err(|failure| error(&failure.to_string()))?
            } else {
                physics
                    .world
                    .angular_velocity(entity)
                    .map_err(|failure| error(&failure.to_string()))?
            };
            return Ok(Value::Vec3(value.map(f64::from)));
        }
        let control = match call {
            Physics3dCall::SetVelocity => BodyControl3d::LinearVelocity(value),
            Physics3dCall::SetAngularVelocity => BodyControl3d::AngularVelocity(value),
            Physics3dCall::ApplyImpulse => BodyControl3d::Impulse(value),
            _ => unreachable!("only motion controls reach here"),
        };
        let outcome = if let Some(body) = authored {
            physics.world.remember_control(entity, body.kind, control)
        } else {
            physics.world.apply_control(entity, control)
        };
        outcome.map_err(|failure| error(&failure.to_string()))?;
        Ok(Value::Unit)
    }

    fn pending_body3d(&self, entity: EntityId, path: &Path) -> Result<RigidBody3d, RuntimeError> {
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        let data = self
            .world
            .get(entity)
            .ok_or_else(|| error("missing 3D body"))?;
        if [
            RigidBody2dComponent::TYPE_NAME,
            Collider2dComponent::TYPE_NAME,
            TilemapCollider2dComponent::TYPE_NAME,
            Character2dComponent::TYPE_NAME,
        ]
        .into_iter()
        .any(|name| data.components.contains_key(name))
        {
            return Err(error("conflicting 2D and 3D physics components"));
        }
        let payload = data
            .components
            .get(RigidBody3dComponent::TYPE_NAME)
            .ok_or_else(|| error("entity has no authored 3D rigid body"))?;
        let mut body: RigidBody3d = serde_json::from_value(payload.clone())
            .map_err(|failure| error(&format!("invalid 3D rigid body: {failure}")))?;
        let payload = data
            .components
            .get(Collider3dComponent::TYPE_NAME)
            .ok_or_else(|| error("entity has no authored 3D collider"))?;
        let collider: Collider3dComponent = serde_json::from_value(payload.clone())
            .map_err(|failure| error(&format!("invalid 3D collider: {failure}")))?;
        if let Some(transform) = self.world.world_transform(entity) {
            body.position = transform.position;
            body.rotation = transform.rotation;
        }
        if body.kind != RigidBodyKind::Static && data.transform_3d.is_some_and(|t| t.z_locked) {
            return Err(error("moving 3D bodies cannot lock depth"));
        }
        PhysicsWorld3d::validate_body(entity, body, &collider.0)
            .map_err(|failure| error(&failure.to_string()))?;
        Ok(body)
    }
}
