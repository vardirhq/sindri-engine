//! Typed copies of classified 3D support, raw movement and platform carry.
use decay_semantic::{Environment, HostType, Type};

pub(crate) const MOTION: &str = "CharacterMotion3d";
pub(crate) const MOVEMENT: &str = "CharacterMovement3d";
pub(crate) const COLLISION: &str = "CharacterCollision3d";
pub(crate) const CARRY: &str = "CharacterCarry3d";
pub(crate) const MOTION_FIELDS: [&str; 7] = [
    "translation",
    "movement",
    "grounded",
    "ground",
    "ground_walkable",
    "ground_started_penetrating",
    "platform",
];
pub(crate) const MOVEMENT_FIELDS: [&str; 4] = [
    "translation",
    "grounded",
    "sliding_down_slope",
    "collisions",
];
pub(crate) const COLLISION_FIELDS: [&str; 3] =
    ["hit", "translation_applied", "translation_remaining"];
pub(crate) const CARRY_FIELDS: [&str; 3] = ["entity", "requested", "motion"];

pub(crate) fn add_snapshots(environment: &mut Environment) {
    let named = |name: &str| Type::Named(name.to_owned());
    let optional = |name: &str| Type::Optional(Box::new(named(name)));
    register(
        environment,
        COLLISION,
        COLLISION_FIELDS,
        [named(super::physics3d::RAY_HIT), Type::Vec3, Type::Vec3],
    );
    register(
        environment,
        MOVEMENT,
        MOVEMENT_FIELDS,
        [
            Type::Vec3,
            Type::Bool,
            Type::Bool,
            Type::array_of(named(COLLISION)),
        ],
    );
    register(
        environment,
        CARRY,
        CARRY_FIELDS,
        [named(super::ENTITY), Type::Vec3, named(MOVEMENT)],
    );
    register(
        environment,
        MOTION,
        MOTION_FIELDS,
        [
            Type::Vec3,
            named(MOVEMENT),
            Type::Bool,
            optional(super::physics3d::RAY_HIT),
            Type::Bool,
            Type::Bool,
            optional(CARRY),
        ],
    );
}

fn register<const N: usize>(
    environment: &mut Environment,
    name: &str,
    fields: [&str; N],
    types: [Type; N],
) {
    let fields: Vec<_> = fields
        .into_iter()
        .zip(types)
        .map(|(name, ty)| (name.to_owned(), ty))
        .collect();
    environment.add_struct(name, fields.clone());
    let mut host = HostType::new();
    for (name, ty) in fields {
        host = host.with_value(name, ty);
    }
    environment.add_type(name, host);
}
