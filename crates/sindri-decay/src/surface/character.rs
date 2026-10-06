//! Names and types shared by controller result registration and runtime copies.

use decay_semantic::{Environment, HostType, Type};

use super::{ENTITY, raycast::RAY_HIT};

pub(crate) const CHARACTER_MOTION: &str = "CharacterMotion2d";
pub(crate) const MOTION_FIELDS: [&str; 18] = [
    "translation",
    "slide_translation",
    "remaining",
    "grounded",
    "ground",
    "ground_walkable",
    "ground_started_penetrating",
    "started_penetrating",
    "iteration_limit_reached",
    "collisions",
    "step_translation",
    "snap_translation",
    "platform",
    "carry_requested",
    "carry_translation",
    "carry_collisions",
    "carry_started_penetrating",
    "carry_iteration_limit_reached",
];

pub(crate) fn add_snapshot(environment: &mut Environment) {
    let hit = || Type::Named(RAY_HIT.to_owned());
    let fields: Vec<_> = MOTION_FIELDS
        .into_iter()
        .zip([
            Type::Vec2,
            Type::Vec2,
            Type::Vec2,
            Type::Bool,
            Type::Optional(Box::new(hit())),
            Type::Bool,
            Type::Bool,
            Type::Bool,
            Type::Bool,
            Type::array_of(hit()),
            Type::Vec2,
            Type::Vec2,
            Type::Named(ENTITY.to_owned()),
            Type::Vec2,
            Type::Vec2,
            Type::array_of(hit()),
            Type::Bool,
            Type::Bool,
        ])
        .map(|(name, ty)| (name.to_owned(), ty))
        .collect();
    environment.add_struct(CHARACTER_MOTION, fields.clone());
    let mut motion = HostType::new();
    for (name, ty) in fields {
        motion = motion.with_value(name, ty);
    }
    environment.add_type(CHARACTER_MOTION, motion);
}
