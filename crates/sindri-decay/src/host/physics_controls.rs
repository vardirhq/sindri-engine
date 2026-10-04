//! Controls that update both a live body and its runtime authored payload.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_physics::{RigidBody2d, RigidBodyKind};

use super::WorldHost;
use crate::surface::PhysicsCall;

const BODY: &str = "sindri.physics2d.rigid_body";

impl WorldHost<'_> {
    pub(super) fn drop_through_call(
        &mut self,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let entity = self.entity_argument(path, args, 0, "the body")?;
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        let seconds = args.get(1).ok_or_else(|| error("needs a duration"))?;
        let seconds = super::convert::number(path, seconds)?;
        #[allow(clippy::cast_possible_truncation)]
        // Backend units are f32; overflow is validated below.
        let seconds = seconds as f32;
        let body: RigidBody2d = self
            .world
            .get(entity)
            .and_then(|data| data.components.get(BODY))
            .ok_or_else(|| error("entity has no authored 2D rigid body"))
            .and_then(|payload| {
                serde_json::from_value(payload.clone())
                    .map_err(|failure| error(&format!("invalid rigid body: {failure}")))
            })?;
        if body.kind != RigidBodyKind::Dynamic {
            return Err(error("drop-through requires a dynamic body"));
        }
        let Some(physics) = self.physics.as_mut() else {
            return Err(error("needs physics, and this host is not running any"));
        };
        let outcome = if physics.world.contains(entity) {
            physics.world.drop_through(entity, seconds)
        } else {
            physics.world.remember_drop_through(entity, seconds)
        };
        outcome.map_err(|failure| error(&failure.to_string()))?;
        Ok(Value::Unit)
    }

    pub(super) fn continuous_collision_call(
        &mut self,
        call: PhysicsCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let entity = self.entity_argument(path, args, 0, "the body")?;
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        let Some(physics) = self.physics.as_mut() else {
            return Err(error("needs physics, and this host is not running any"));
        };
        let payload = self
            .world
            .get(entity)
            .and_then(|data| data.components.get(BODY))
            .ok_or_else(|| error("entity has no authored 2D rigid body"))?;
        let body: RigidBody2d = serde_json::from_value(payload.clone())
            .map_err(|failure| error(&format!("invalid rigid body: {failure}")))?;
        if matches!(call, PhysicsCall::ContinuousCollision) {
            let enabled = if physics.world.contains(entity) {
                physics
                    .world
                    .continuous_collision(entity)
                    .map_err(|failure| error(&failure.to_string()))?
            } else {
                body.continuous_collision
            };
            return Ok(Value::Bool(enabled));
        }
        if body.kind != RigidBodyKind::Dynamic {
            return Err(error(
                "continuous collision control requires a dynamic body",
            ));
        }
        let Some(Value::Bool(enabled)) = args.get(1) else {
            return Err(error("continuous collision setting must be a bool"));
        };
        if physics.world.contains(entity) {
            physics
                .world
                .set_continuous_collision(entity, *enabled)
                .map_err(|failure| error(&failure.to_string()))?;
        }
        // Preserve this setting through the valid spawn-to-synchronization window.
        let payload = self
            .world
            .get_mut(entity)
            .and_then(|data| data.components.get_mut(BODY))
            .ok_or_else(|| error("entity has no authored 2D rigid body"))?;
        payload["continuous_collision"] = serde_json::Value::Bool(*enabled);
        Ok(Value::Unit)
    }
}
