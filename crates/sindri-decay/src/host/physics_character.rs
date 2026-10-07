//! Controller input crosses into the scene queue; results cross out as copies.

use std::rc::Rc;

use decay_ir::{Path, StructShape};
use decay_runtime::{RuntimeError, Value};
use sindri_core::{EntityId, SceneComponent, World};
use sindri_physics::{GroundedSlideMotion2d, RayHit2d, ShapeHit2d};
use sindri_scene::{Character2dComponent, RigidBody2dComponent};

use super::{WorldHost, raycast};
use crate::surface::{
    PhysicsCall,
    character::{CHARACTER_MOTION, MOTION_FIELDS},
};

impl WorldHost<'_> {
    pub(super) fn physics_character(
        &mut self,
        call: PhysicsCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let entity = self.entity_argument(path, args, 0, "the character")?;
        if matches!(call, PhysicsCall::CharacterMotion) {
            let characters = self
                .characters
                .as_ref()
                .ok_or_else(|| no_characters(path))?;
            if !self.world.is_active(entity) || !self.has_character(entity) {
                return Ok(Value::Null);
            }
            return Ok(characters
                .motions
                .get(entity)
                .map_or(Value::Null, |motion| snapshot(self.world, motion)));
        }
        self.validate_character(entity, path)?;
        let displacement = raycast::vector(path, args.get(1))?;
        let Some(Value::Bool(snap)) = args.get(2) else {
            return Err(error(path, "snap must be bool"));
        };
        self.characters
            .as_mut()
            .ok_or_else(|| no_characters(path))?
            .requests
            .move_character(entity, displacement, *snap)
            .map_err(|failure| error(path, &failure.to_string()))?;
        Ok(Value::Unit)
    }

    pub(super) fn character_drop_through(
        &mut self,
        entity: EntityId,
        seconds: f32,
        path: &Path,
    ) -> Result<Value, RuntimeError> {
        self.validate_character(entity, path)?;
        self.characters
            .as_mut()
            .ok_or_else(|| no_characters(path))?
            .requests
            .drop_through(entity, seconds)
            .map_err(|failure| error(path, &failure.to_string()))?;
        Ok(Value::Unit)
    }

    fn has_character(&self, entity: EntityId) -> bool {
        self.world.get(entity).is_some_and(|data| {
            data.components
                .contains_key(Character2dComponent::TYPE_NAME)
        })
    }

    fn validate_character(&self, entity: EntityId, path: &Path) -> Result<(), RuntimeError> {
        if !self.world.is_active(entity) {
            return Err(error(path, "character must be active"));
        }
        let data = self
            .world
            .get(entity)
            .ok_or_else(|| error(path, "character no longer exists"))?;
        let payload = data
            .components
            .get(Character2dComponent::TYPE_NAME)
            .ok_or_else(|| error(path, "entity has no authored Character 2D"))?;
        serde_json::from_value::<Character2dComponent>(payload.clone())
            .map_err(|failure| error(path, &format!("invalid character: {failure}")))?;
        if data
            .components
            .contains_key(RigidBody2dComponent::TYPE_NAME)
        {
            return Err(error(
                path,
                "a character owns its body; remove the rigid body component",
            ));
        }
        if self.world.world_transform(entity).is_none() {
            return Err(error(path, "a character requires a transform"));
        }
        // Probe assembly, including generated tilemap pieces, is validated by
        // scene synchronization before any queued motion can be applied.
        Ok(())
    }
}

fn error(path: &Path, message: &str) -> RuntimeError {
    RuntimeError::Host(format!("{}: {message}", path.dotted()))
}

fn no_characters(path: &Path) -> RuntimeError {
    error(
        path,
        "needs scene character controllers, and this host is not running any",
    )
}

fn hit_snapshot(hit: ShapeHit2d) -> Value {
    raycast::snapshot(RayHit2d {
        entity: hit.entity,
        point: hit.point,
        normal: hit.normal,
        distance: hit.distance,
    })
}

fn hits(world: &World, hits: &[ShapeHit2d]) -> Value {
    Value::array(
        hits.iter()
            .filter(|hit| world.is_active(hit.entity))
            .copied()
            .map(hit_snapshot)
            .collect(),
    )
}

fn snapshot(world: &World, motion: &GroundedSlideMotion2d) -> Value {
    let ground = motion.ground.hit.filter(|hit| world.is_active(hit.entity));
    let carry = motion.platform.as_ref();
    Value::Struct {
        shape: Rc::new(StructShape {
            name: CHARACTER_MOTION.to_owned(),
            fields: MOTION_FIELDS.into_iter().map(str::to_owned).collect(),
        }),
        fields: Rc::new(vec![
            Value::Vec2(motion.translation.map(f64::from)),
            Value::Vec2(motion.slide.translation.map(f64::from)),
            Value::Vec2(motion.slide.remaining.map(f64::from)),
            Value::Bool(motion.grounded && ground.is_some()),
            ground.map_or(Value::Null, hit_snapshot),
            Value::Bool(motion.ground.walkable && ground.is_some()),
            Value::Bool(motion.ground.started_penetrating),
            Value::Bool(motion.slide.started_penetrating),
            Value::Bool(motion.slide.iteration_limit_reached),
            hits(world, &motion.slide.collisions),
            Value::Vec2(motion.step_translation.map(f64::from)),
            Value::Vec2(motion.snap_translation.map(f64::from)),
            carry
                .filter(|carry| world.is_active(carry.entity))
                .map_or(Value::Null, |carry| {
                    Value::Reference(carry.entity.to_bits())
                }),
            Value::Vec2(
                carry
                    .map_or([0.0; 2], |carry| carry.requested_translation)
                    .map(f64::from),
            ),
            Value::Vec2(
                carry
                    .map_or([0.0; 2], |carry| carry.motion.translation)
                    .map(f64::from),
            ),
            carry.map_or_else(
                || Value::array(Vec::new()),
                |carry| hits(world, &carry.motion.collisions),
            ),
            Value::Bool(carry.is_some_and(|carry| carry.motion.started_penetrating)),
            Value::Bool(carry.is_some_and(|carry| carry.motion.iteration_limit_reached)),
        ]),
    }
}
