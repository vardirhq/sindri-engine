//! Typed controls persist in the runtime component until scene synchronization.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::SceneComponent;
use sindri_scene::HingeJoint2dComponent;

use super::WorldHost;
use super::convert::number;

impl WorldHost<'_> {
    pub(super) fn hinge_motor_call(
        &mut self,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let owner = self.entity_argument(path, args, 0, "the hinge owner")?;
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        if self.physics.is_none() {
            return Err(error("needs physics, and this host is not running any"));
        }
        let velocity = number(path, args.get(1).unwrap_or(&Value::Null))?;
        let torque = number(path, args.get(2).unwrap_or(&Value::Null))?;
        let payload = self
            .world
            .get(owner)
            .and_then(|data| data.components.get(HingeJoint2dComponent::TYPE_NAME))
            .ok_or_else(|| error("entity has no authored 2D hinge joint"))?;
        let mut hinge: HingeJoint2dComponent = serde_json::from_value(payload.clone())
            .map_err(|failure| error(&format!("invalid hinge: {failure}")))?;
        #[allow(clippy::cast_possible_truncation)]
        // Physics uses f32; finite/overflow validation happens before mutation.
        let (velocity, torque) = (velocity as f32, torque as f32);
        hinge.settings.motor_velocity = velocity;
        hinge.settings.motor_max_torque = torque;
        hinge.settings.motor_enabled = torque > 0.0;
        hinge
            .settings
            .validate()
            .map_err(|failure| error(&failure.to_string()))?;
        // Preserve unknown component fields. The next synchronization applies
        // the motor, including when endpoints have not materialized yet.
        let payload = self
            .world
            .get_mut(owner)
            .and_then(|data| data.components.get_mut(HingeJoint2dComponent::TYPE_NAME))
            .ok_or_else(|| error("entity has no authored 2D hinge joint"))?;
        payload["motor_velocity"] = serde_json::json!(velocity);
        payload["motor_max_torque"] = serde_json::json!(torque);
        payload["motor_enabled"] = serde_json::json!(torque > 0.0);
        Ok(Value::Unit)
    }
}
