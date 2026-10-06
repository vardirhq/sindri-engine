//! Copied controller result reference.
use super::{TypeEntry, value};

pub(super) const CHARACTER_MOTION: TypeEntry = TypeEntry {
    name: "CharacterMotion2d",
    text: "A copied result of scene-owned 2D character movement. Gameplay reads support and collision flags to choose its own gravity, jump and recovery policy. This is historical movement, not a live solver contact or a pending request.",
    members: &[
        value(
            "translation",
            "Total world displacement applied once at the last fixed step, including slide, step, snap and platform carry.",
        ),
        value(
            "slide_translation",
            "Selected collision-limited slide displacement, after carry or from the lifted pose of an accepted step.",
        ),
        value(
            "remaining",
            "Unsatisfied slide displacement when the iteration budget is exhausted; collision-blocked normal motion is discarded.",
        ),
        value(
            "grounded",
            "Whether the completed endpoint has walkable support and the request was not ascending or penetrating. False if the ground entity is now inactive or removed.",
        ),
        value(
            "ground",
            "Nearest support-query RayHit2d before landing/snap, or null when absent or inactive. Distance describes that downward travel.",
        ),
        value(
            "ground_walkable",
            "Whether the ground hit meets the authored up/slope limit; false when the hit is absent or filtered.",
        ),
        value(
            "ground_started_penetrating",
            "Whether the support probe started in strict penetration.",
        ),
        value(
            "started_penetrating",
            "Whether the selected character slide started in strict penetration; carry reports its own flag.",
        ),
        value(
            "iteration_limit_reached",
            "Whether the selected slide exhausted its bounded iteration budget.",
        ),
        value(
            "collisions",
            "Copied ordered slide RayHit2d values, filtered to active existing entities. Distances belong to each sweep phase.",
        ),
        value(
            "step_translation",
            "Accepted step lift, or zero for ordinary movement.",
        ),
        value(
            "snap_translation",
            "Additional downward landing/snap translation, or zero.",
        ),
        value(
            "platform",
            "Verified carrying entity, or null without carry or when it is now inactive/removed.",
        ),
        value(
            "carry_requested",
            "Platform translation/rotation-point displacement requested for carry, or zero without carry. Remains historical even when the platform reference is filtered.",
        ),
        value(
            "carry_translation",
            "Collision-limited carry displacement included once in translation, or zero without carry.",
        ),
        value(
            "carry_collisions",
            "Copied ordered carry RayHit2d values, filtered to active existing entities.",
        ),
        value(
            "carry_started_penetrating",
            "Whether the carry sweep started in strict penetration; false without carry.",
        ),
        value(
            "carry_iteration_limit_reached",
            "Whether the carry sweep exhausted its iteration budget; false without carry.",
        ),
    ],
};
