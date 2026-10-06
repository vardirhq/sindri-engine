//! Reversible constraint suspension and validated distance tuning.
use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::{EntityId, SceneComponent};
use sindri_scene::{
    DistanceJoint2dComponent, HingeJoint2dComponent, SliderJoint2dComponent, SpringJoint2dComponent,
};

use super::WorldHost;
use super::convert::{as_f32, number};
use crate::surface::PhysicsCall;

const KINDS: [&str; 4] = [
    DistanceJoint2dComponent::TYPE_NAME,
    HingeJoint2dComponent::TYPE_NAME,
    SliderJoint2dComponent::TYPE_NAME,
    SpringJoint2dComponent::TYPE_NAME,
];

impl WorldHost<'_> {
    pub(super) fn joint_state_call(
        &mut self,
        call: PhysicsCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let owner = self.entity_argument(path, args, 0, "the joint owner")?;
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        if self.physics.is_none() {
            return Err(error("needs physics, and this host is not running any"));
        }
        let (kind, enabled) = self.joint_state(owner, path)?;
        match call {
            PhysicsCall::RemoveJoint => {
                self.world
                    .get_mut(owner)
                    .expect("checked owner")
                    .components
                    .remove(kind);
                Ok(Value::Unit)
            }
            PhysicsCall::JointEnabled => Ok(Value::Bool(enabled)),
            PhysicsCall::SetJointEnabled => {
                let Some(Value::Bool(enabled)) = args.get(1) else {
                    return Err(error("joint enabled setting must be a bool"));
                };
                self.world
                    .get_mut(owner)
                    .expect("checked owner")
                    .components
                    .get_mut(kind)
                    .expect("checked joint")["enabled"] = serde_json::json!(enabled);
                Ok(Value::Unit)
            }
            PhysicsCall::SetDistance => {
                if kind != DistanceJoint2dComponent::TYPE_NAME {
                    return Err(error("entity has no authored 2D distance joint"));
                }
                let distance = as_f32(number(path, args.get(1).unwrap_or(&Value::Null))?);
                DistanceJoint2dComponent {
                    max_distance: distance,
                    ..DistanceJoint2dComponent::default()
                }
                .validate()
                .map_err(|failure| error(&failure.to_string()))?;
                self.world
                    .get_mut(owner)
                    .expect("checked owner")
                    .components
                    .get_mut(kind)
                    .expect("checked joint")["max_distance"] = serde_json::json!(distance);
                Ok(Value::Unit)
            }
            _ => unreachable!("only joint state calls reach here"),
        }
    }

    pub(super) fn joint_state(
        &self,
        owner: EntityId,
        path: &Path,
    ) -> Result<(&'static str, bool), RuntimeError> {
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        let components = &self.world.get(owner).expect("checked owner").components;
        let mut kinds = KINDS
            .into_iter()
            .filter(|kind| components.contains_key(*kind));
        let kind = kinds
            .next()
            .ok_or_else(|| error("entity has no authored 2D joint"))?;
        if kinds.next().is_some() {
            return Err(error("entity has conflicting 2D joint components"));
        }
        let payload = components[kind].clone();
        let invalid = |failure: &dyn std::fmt::Display| error(&format!("invalid joint: {failure}"));
        let enabled = match kind {
            DistanceJoint2dComponent::TYPE_NAME => {
                let joint: DistanceJoint2dComponent =
                    serde_json::from_value(payload).map_err(|failure| invalid(&failure))?;
                joint.validate().map_err(|failure| invalid(&failure))?;
                joint.enabled
            }
            HingeJoint2dComponent::TYPE_NAME => {
                let joint: HingeJoint2dComponent =
                    serde_json::from_value(payload).map_err(|failure| invalid(&failure))?;
                joint
                    .settings
                    .validate()
                    .map_err(|failure| invalid(&failure))?;
                joint.enabled
            }
            SliderJoint2dComponent::TYPE_NAME => {
                let joint: SliderJoint2dComponent =
                    serde_json::from_value(payload).map_err(|failure| invalid(&failure))?;
                joint
                    .settings
                    .validate()
                    .map_err(|failure| invalid(&failure))?;
                joint.enabled
            }
            _ => {
                let joint: SpringJoint2dComponent =
                    serde_json::from_value(payload).map_err(|failure| invalid(&failure))?;
                joint
                    .settings
                    .validate()
                    .map_err(|failure| invalid(&failure))?;
                joint.enabled
            }
        };
        Ok((kind, enabled))
    }
}
