//! Position-drive settings persist in the authored slider until synchronization.
use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::SceneComponent;
use sindri_physics::MotorMode2d;
use sindri_scene::SliderJoint2dComponent;

use super::WorldHost;
use super::convert::{as_f32, number};

impl WorldHost<'_> {
    pub(super) fn slider_position_motor_call(
        &mut self,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let owner = self.entity_argument(path, args, 0, "the slider owner")?;
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        if self.physics.is_none() {
            return Err(error("needs physics, and this host is not running any"));
        }
        let payload = self
            .world
            .get(owner)
            .and_then(|data| data.components.get(SliderJoint2dComponent::TYPE_NAME))
            .ok_or_else(|| error("entity has no authored 2D slider joint"))?;
        let mut slider: SliderJoint2dComponent = serde_json::from_value(payload.clone())
            .map_err(|failure| error(&format!("invalid slider: {failure}")))?;
        let scalar = |index| number(path, args.get(index).unwrap_or(&Value::Null)).map(as_f32);
        slider.settings.motor_mode = MotorMode2d::Position;
        slider.settings.motor_target_distance = scalar(1)?;
        slider.settings.motor_stiffness = scalar(2)?;
        slider.settings.motor_damping = scalar(3)?;
        slider.settings.motor_max_force = scalar(4)?;
        slider.settings.motor_enabled = slider.settings.motor_max_force > 0.0;
        slider
            .settings
            .validate()
            .map_err(|failure| error(&failure.to_string()))?;
        let payload = self
            .world
            .get_mut(owner)
            .and_then(|data| data.components.get_mut(SliderJoint2dComponent::TYPE_NAME))
            .ok_or_else(|| error("entity has no authored 2D slider joint"))?;
        payload["motor_mode"] = serde_json::json!("position");
        payload["motor_target_distance"] = serde_json::json!(slider.settings.motor_target_distance);
        payload["motor_stiffness"] = serde_json::json!(slider.settings.motor_stiffness);
        payload["motor_damping"] = serde_json::json!(slider.settings.motor_damping);
        payload["motor_max_force"] = serde_json::json!(slider.settings.motor_max_force);
        payload["motor_enabled"] = serde_json::json!(slider.settings.motor_enabled);
        Ok(Value::Unit)
    }
}
