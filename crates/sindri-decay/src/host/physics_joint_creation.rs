//! Creation authors a component; scene synchronization owns solver allocation.
use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::{EntityId, SceneComponent};
use sindri_physics::HingeSettings2d;
use sindri_scene::{DistanceJoint2dComponent, HingeJoint2dComponent};

use super::WorldHost;
use super::convert::{as_f32, number};
use super::physics_joint_state::KINDS;
use super::raycast::vector;

impl WorldHost<'_> {
    pub(super) fn create_distance_joint_call(
        &mut self,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let owner = self.empty_joint_owner(path, args)?;
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        let (first, second) = self.joint_endpoint_references(owner, path, args)?;
        let max_distance = as_f32(number(path, args.get(3).unwrap_or(&Value::Null))?);
        DistanceJoint2dComponent {
            max_distance,
            ..DistanceJoint2dComponent::default()
        }
        .validate()
        .map_err(|failure| error(&failure.to_string()))?;
        self.world
            .get_mut(owner)
            .expect("checked owner")
            .components
            .insert(
                DistanceJoint2dComponent::TYPE_NAME.into(),
                serde_json::json!({
                    "first": first,
                    "second": second,
                    "enabled": true,
                    "max_distance": max_distance,
                }),
            );
        Ok(Value::Unit)
    }

    pub(super) fn create_hinge_joint_call(
        &mut self,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let owner = self.empty_joint_owner(path, args)?;
        let (first, second) = self.joint_endpoint_references(owner, path, args)?;
        let settings = HingeSettings2d {
            first_anchor: vector(path, args.get(3))?,
            second_anchor: vector(path, args.get(4))?,
            ..HingeSettings2d::default()
        };
        settings
            .validate()
            .map_err(|failure| RuntimeError::Host(format!("{}: {failure}", path.dotted())))?;
        let payload = serde_json::to_value(HingeJoint2dComponent {
            first,
            second,
            enabled: true,
            settings,
        })
        .map_err(|failure| RuntimeError::Host(format!("{}: {failure}", path.dotted())))?;
        self.world
            .get_mut(owner)
            .expect("checked owner")
            .components
            .insert(HingeJoint2dComponent::TYPE_NAME.into(), payload);
        Ok(Value::Unit)
    }

    fn empty_joint_owner(&self, path: &Path, args: &[Value]) -> Result<EntityId, RuntimeError> {
        let owner = self.entity_argument(path, args, 0, "the joint owner")?;
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        if self.physics.is_none() {
            return Err(error("needs physics, and this host is not running any"));
        }
        let components = &self.world.get(owner).expect("checked owner").components;
        if KINDS.iter().any(|kind| components.contains_key(*kind)) {
            return Err(error("owner already has an authored 2D joint"));
        }
        Ok(owner)
    }
}
