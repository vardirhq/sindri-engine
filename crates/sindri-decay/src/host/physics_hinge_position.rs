//! Position-drive settings persist in the authored hinge until synchronization.
use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::SceneComponent;
use sindri_physics::MotorMode2d;
use sindri_scene::HingeJoint2dComponent;

use super::WorldHost;
use super::convert::{as_f32, number};

impl WorldHost<'_> {
    pub(super) fn hinge_position_motor_call(
        &mut self,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let owner = self.entity_argument(path, args, 0, "the hinge owner")?;
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        if self.physics.is_none() {
            return Err(error("needs physics, and this host is not running any"));
        }
        let payload = self
            .world
            .get(owner)
            .and_then(|data| data.components.get(HingeJoint2dComponent::TYPE_NAME))
            .ok_or_else(|| error("entity has no authored 2D hinge joint"))?;
        let mut hinge: HingeJoint2dComponent = serde_json::from_value(payload.clone())
            .map_err(|failure| error(&format!("invalid hinge: {failure}")))?;
        let scalar = |index| number(path, args.get(index).unwrap_or(&Value::Null)).map(as_f32);
        hinge.settings.motor_mode = MotorMode2d::Position;
        hinge.settings.motor_target_angle = scalar(1)?;
        hinge.settings.motor_stiffness = scalar(2)?;
        hinge.settings.motor_damping = scalar(3)?;
        hinge.settings.motor_max_torque = scalar(4)?;
        hinge.settings.motor_enabled = hinge.settings.motor_max_torque > 0.0;
        hinge
            .settings
            .validate()
            .map_err(|failure| error(&failure.to_string()))?;
        let payload = self
            .world
            .get_mut(owner)
            .expect("checked owner")
            .components
            .get_mut(HingeJoint2dComponent::TYPE_NAME)
            .expect("checked hinge");
        payload["motor_mode"] = serde_json::json!("position");
        payload["motor_target_angle"] = serde_json::json!(hinge.settings.motor_target_angle);
        payload["motor_stiffness"] = serde_json::json!(hinge.settings.motor_stiffness);
        payload["motor_damping"] = serde_json::json!(hinge.settings.motor_damping);
        payload["motor_max_torque"] = serde_json::json!(hinge.settings.motor_max_torque);
        payload["motor_enabled"] = serde_json::json!(hinge.settings.motor_enabled);
        Ok(Value::Unit)
    }
}
