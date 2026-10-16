//! Historical movement remains copied; stale world references are filtered.
use std::rc::Rc;

use decay_ir::StructShape;
use decay_runtime::Value;
use sindri_core::World;
use sindri_physics::{CharacterMotion3d, GroundedCharacterMotion3d, RayHit3d, ShapeHit3d};

use crate::surface::character3d::{
    CARRY, CARRY_FIELDS, COLLISION, COLLISION_FIELDS, MOTION, MOTION_FIELDS, MOVEMENT,
    MOVEMENT_FIELDS,
};

pub(super) fn snapshot(world: &World, motion: &GroundedCharacterMotion3d) -> Value {
    let ground = motion.ground.hit.filter(|hit| world.is_active(hit.entity));
    let platform = motion
        .platform
        .as_ref()
        .filter(|carry| world.is_active(carry.entity));
    record(
        MOTION,
        MOTION_FIELDS,
        vec![
            vector(motion.translation),
            movement(world, &motion.movement),
            Value::Bool(motion.grounded && ground.is_some()),
            ground.map_or(Value::Null, hit),
            Value::Bool(motion.ground.walkable && ground.is_some()),
            Value::Bool(motion.ground.started_penetrating),
            platform.map_or(Value::Null, |carry| {
                record(
                    CARRY,
                    CARRY_FIELDS,
                    vec![
                        Value::Reference(carry.entity.to_bits()),
                        vector(carry.requested),
                        movement(world, &carry.motion),
                    ],
                )
            }),
        ],
    )
}

fn movement(world: &World, motion: &CharacterMotion3d) -> Value {
    let collisions = motion
        .collisions
        .iter()
        .filter(|collision| world.is_active(collision.hit.entity))
        .map(|collision| {
            record(
                COLLISION,
                COLLISION_FIELDS,
                vec![
                    hit(collision.hit),
                    vector(collision.translation_applied),
                    vector(collision.translation_remaining),
                ],
            )
        })
        .collect();
    record(
        MOVEMENT,
        MOVEMENT_FIELDS,
        vec![
            vector(motion.translation),
            Value::Bool(motion.grounded),
            Value::Bool(motion.sliding_down_slope),
            Value::array(collisions),
        ],
    )
}

fn hit(hit: ShapeHit3d) -> Value {
    super::physics3d_query::snapshot(RayHit3d {
        entity: hit.entity,
        point: hit.point,
        normal: hit.normal,
        distance: hit.distance,
    })
}

fn vector(parts: [f32; 3]) -> Value {
    Value::Vec3(parts.map(f64::from))
}

fn record<const N: usize>(name: &str, names: [&str; N], fields: Vec<Value>) -> Value {
    Value::Struct {
        shape: Rc::new(StructShape {
            name: name.to_owned(),
            fields: names.into_iter().map(str::to_owned).collect(),
        }),
        fields: Rc::new(fields),
    }
}
