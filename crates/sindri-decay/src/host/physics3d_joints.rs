//! Authored 3D rope controls: length and suspension, validated before writing.
//!
//! Like the 2D joint controls, these edit the owner's authored component; the
//! scene driver applies it at the next fixed synchronization, so a script may
//! tune a rope before its bodies exist and every change is replayable.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::SceneComponent;
use sindri_scene::DistanceJoint3dComponent;

use super::WorldHost;
use super::convert::{as_f32, number};
use crate::surface::physics3d::Physics3dCall;

impl WorldHost<'_> {
    pub(super) fn physics3d_joint(
        &mut self,
        call: Physics3dCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        if self.physics3d.is_none() {
            return Err(error("needs 3D physics, and this host is not running any"));
        }
        let owner = self.entity_argument(path, args, 0, "the joint owner")?;
        let kind = DistanceJoint3dComponent::TYPE_NAME;
        let payload = self
            .world
            .get(owner)
            .and_then(|data| data.components.get(kind))
            .ok_or_else(|| error("entity has no authored 3D distance joint"))?;
        let mut joint: DistanceJoint3dComponent = serde_json::from_value(payload.clone())
            .map_err(|failure| error(&format!("invalid joint: {failure}")))?;
        joint
            .validate()
            .map_err(|failure| error(&format!("invalid joint: {failure}")))?;
        let (field, value) = match call {
            Physics3dCall::JointEnabled => return Ok(Value::Bool(joint.enabled)),
            Physics3dCall::SetJointEnabled => {
                let Some(Value::Bool(enabled)) = args.get(1) else {
                    return Err(error("joint enabled setting must be a bool"));
                };
                ("enabled", serde_json::json!(enabled))
            }
            Physics3dCall::SetDistance => {
                let distance = as_f32(number(path, args.get(1).unwrap_or(&Value::Null))?);
                joint.max_distance = distance;
                joint
                    .validate()
                    .map_err(|failure| error(&failure.to_string()))?;
                ("max_distance", serde_json::json!(distance))
            }
            _ => unreachable!("only 3D joint calls reach here"),
        };
        self.world
            .get_mut(owner)
            .expect("checked owner")
            .components
            .get_mut(kind)
            .expect("checked joint")[field] = value;
        Ok(Value::Unit)
    }
}
