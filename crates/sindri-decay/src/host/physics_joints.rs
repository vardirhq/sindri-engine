//! Typed controls persist in the runtime component until scene synchronization.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::SceneComponent;
use sindri_scene::{HingeJoint2dComponent, SliderJoint2dComponent, SpringJoint2dComponent};

use super::WorldHost;
use super::convert::number;
use crate::surface::PhysicsCall;

impl WorldHost<'_> {
    pub(super) fn physics_joint_call(
        &mut self,
        call: PhysicsCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        match call {
            PhysicsCall::CreateDistanceJoint => self.create_distance_joint_call(path, args),
            PhysicsCall::CreateHingeJoint => self.create_hinge_joint_call(path, args),
            PhysicsCall::SetJointEndpoints => self.joint_endpoints_call(path, args),
            PhysicsCall::SetHingeMotor => self.hinge_motor_call(path, args),
            PhysicsCall::SetSliderMotor => self.slider_motor_call(path, args),
            PhysicsCall::SetSpring => self.spring_call(path, args),
            PhysicsCall::JointEnabled
            | PhysicsCall::SetJointEnabled
            | PhysicsCall::SetDistance
            | PhysicsCall::RemoveJoint => self.joint_state_call(call, path, args),
            _ => unreachable!("only authored joint controls reach here"),
        }
    }

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
    pub(super) fn slider_motor_call(
        &mut self,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let owner = self.entity_argument(path, args, 0, "the slider owner")?;
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        if self.physics.is_none() {
            return Err(error("needs physics, and this host is not running any"));
        }
        let velocity = number(path, args.get(1).unwrap_or(&Value::Null))?;
        let force = number(path, args.get(2).unwrap_or(&Value::Null))?;
        let payload = self
            .world
            .get(owner)
            .and_then(|data| data.components.get(SliderJoint2dComponent::TYPE_NAME))
            .ok_or_else(|| error("entity has no authored 2D slider joint"))?;
        let mut slider: SliderJoint2dComponent = serde_json::from_value(payload.clone())
            .map_err(|failure| error(&format!("invalid slider: {failure}")))?;
        #[allow(clippy::cast_possible_truncation)]
        // Backend units are f32; validate overflow before mutation.
        let (velocity, force) = (velocity as f32, force as f32);
        slider.settings.motor_velocity = velocity;
        slider.settings.motor_max_force = force;
        slider.settings.motor_enabled = force > 0.0;
        slider
            .settings
            .validate()
            .map_err(|failure| error(&failure.to_string()))?;
        let payload = self
            .world
            .get_mut(owner)
            .and_then(|data| data.components.get_mut(SliderJoint2dComponent::TYPE_NAME))
            .ok_or_else(|| error("entity has no authored 2D slider joint"))?;
        payload["motor_velocity"] = serde_json::json!(velocity);
        payload["motor_max_force"] = serde_json::json!(force);
        payload["motor_enabled"] = serde_json::json!(force > 0.0);
        Ok(Value::Unit)
    }

    pub(super) fn spring_call(
        &mut self,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let owner = self.entity_argument(path, args, 0, "the spring owner")?;
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        if self.physics.is_none() {
            return Err(error("needs physics, and this host is not running any"));
        }
        let length = number(path, args.get(1).unwrap_or(&Value::Null))?;
        let stiffness = number(path, args.get(2).unwrap_or(&Value::Null))?;
        let damping = number(path, args.get(3).unwrap_or(&Value::Null))?;
        let payload = self
            .world
            .get(owner)
            .and_then(|data| data.components.get(SpringJoint2dComponent::TYPE_NAME))
            .ok_or_else(|| error("entity has no authored 2D spring joint"))?;
        let mut spring: SpringJoint2dComponent = serde_json::from_value(payload.clone())
            .map_err(|failure| error(&format!("invalid spring: {failure}")))?;
        #[allow(clippy::cast_possible_truncation)]
        // Backend units are f32; validate overflow before mutation.
        let (length, stiffness, damping) = (length as f32, stiffness as f32, damping as f32);
        spring.settings.rest_length = length;
        spring.settings.stiffness = stiffness;
        spring.settings.damping = damping;
        spring
            .settings
            .validate()
            .map_err(|failure| error(&failure.to_string()))?;
        let payload = self
            .world
            .get_mut(owner)
            .and_then(|data| data.components.get_mut(SpringJoint2dComponent::TYPE_NAME))
            .ok_or_else(|| error("entity has no authored 2D spring joint"))?;
        payload["rest_length"] = serde_json::json!(length);
        payload["stiffness"] = serde_json::json!(stiffness);
        payload["damping"] = serde_json::json!(damping);
        Ok(Value::Unit)
    }
}
