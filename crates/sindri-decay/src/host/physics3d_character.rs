//! Scripts queue scene-owned displacement; they never drive a second solver.
use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::{EntityId, SceneComponent};
use sindri_scene::{Character3dComponent, RigidBody2dComponent, RigidBody3dComponent};

use super::{WorldHost, physics3d::vector, physics3d_character_snapshot::snapshot};
use crate::surface::physics3d::Physics3dCall;

impl WorldHost<'_> {
    pub(super) fn physics3d_character(
        &mut self,
        call: Physics3dCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let reading = matches!(call, Physics3dCall::CharacterMotion);
        if args.len() != if reading { 1 } else { 3 } {
            return Err(error(path, "incorrect argument count"));
        }
        let entity = self.entity_argument(path, args, 0, "the character")?;
        if reading {
            let characters = self
                .characters3d
                .as_ref()
                .ok_or_else(|| error(path, "host has no scene 3D character controllers"))?;
            if !self.world.is_active(entity) || !self.has_character3d(entity) {
                return Ok(Value::Null);
            }
            return Ok(characters
                .motions
                .get(entity)
                .map_or(Value::Null, |motion| snapshot(self.world, motion)));
        }
        self.validate_character3d(entity, path)?;
        let displacement = vector(path, &args[1])?;
        let Value::Bool(snap) = args[2] else {
            return Err(error(path, "snap must be bool"));
        };
        self.characters3d
            .as_mut()
            .ok_or_else(|| error(path, "host has no scene 3D character controllers"))?
            .requests
            .move_character(entity, displacement, snap)
            .map_err(|failure| error(path, &failure.to_string()))?;
        Ok(Value::Unit)
    }

    fn has_character3d(&self, entity: EntityId) -> bool {
        self.world.get(entity).is_some_and(|data| {
            data.components
                .contains_key(Character3dComponent::TYPE_NAME)
        })
    }

    fn validate_character3d(&self, entity: EntityId, path: &Path) -> Result<(), RuntimeError> {
        if !self.world.is_active(entity) {
            return Err(error(path, "character must be active"));
        }
        let data = self
            .world
            .get(entity)
            .ok_or_else(|| error(path, "character no longer exists"))?;
        let payload = data
            .components
            .get(Character3dComponent::TYPE_NAME)
            .ok_or_else(|| error(path, "entity has no authored Character 3D"))?;
        serde_json::from_value::<Character3dComponent>(payload.clone())
            .map_err(|failure| error(path, &format!("invalid character: {failure}")))?;
        if data
            .components
            .contains_key(RigidBody3dComponent::TYPE_NAME)
            || data
                .components
                .contains_key(RigidBody2dComponent::TYPE_NAME)
            || data
                .components
                .contains_key(sindri_scene::Character2dComponent::TYPE_NAME)
        {
            return Err(error(
                path,
                "a 3D character owns its body; remove competing body/controller components",
            ));
        }
        if self.world.world_transform(entity).is_none() {
            return Err(error(path, "a character requires a transform"));
        }
        // Scaled solid probe assembly is validated atomically by scene sync.
        Ok(())
    }
}

fn error(path: &Path, message: &str) -> RuntimeError {
    RuntimeError::Host(format!("{}: {message}", path.dotted()))
}
