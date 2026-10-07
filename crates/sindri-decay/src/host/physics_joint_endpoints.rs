//! Retargeting writes scoped authored references, never serialized runtime handles.
use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::EntityId;

use super::WorldHost;

impl WorldHost<'_> {
    pub(super) fn joint_endpoints_call(
        &mut self,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let owner = self.entity_argument(path, args, 0, "the joint owner")?;
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        if self.physics.is_none() {
            return Err(error("needs physics, and this host is not running any"));
        }
        let (kind, _) = self.joint_state(owner, path)?;
        let (first_ref, second_ref) = self.joint_endpoint_references(owner, path, args)?;
        let payload = self
            .world
            .get_mut(owner)
            .expect("checked owner")
            .components
            .get_mut(kind)
            .expect("checked joint");
        payload["first"] = serde_json::json!(first_ref);
        payload["second"] = serde_json::json!(second_ref);
        Ok(Value::Unit)
    }

    pub(super) fn joint_endpoint_references(
        &self,
        owner: EntityId,
        path: &Path,
        args: &[Value],
    ) -> Result<(String, String), RuntimeError> {
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        let endpoint = |index, role| {
            if matches!(args.get(index), Some(Value::Null)) {
                return Ok((None, String::new()));
            }
            let entity = self.entity_argument(path, args, index, role)?;
            let reference = self.world.entity_reference(owner, entity).ok_or_else(|| {
                error("endpoint has no stable reference in the owner's scene or prefab")
            })?;
            Ok((Some(entity), reference))
        };
        let (first, first_ref) = endpoint(1, "the first endpoint")?;
        let (second, second_ref) = endpoint(2, "the second endpoint")?;
        if first.is_some() && first == second {
            return Err(error("joint endpoints must be different entities"));
        }
        Ok((first_ref, second_ref))
    }
}
