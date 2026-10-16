//! Copied 3D controller values retain the engine's raw/classified distinction.
use super::{TypeEntry, value};

pub(super) const TYPES: &[TypeEntry] = &[
    TypeEntry {
        name: "CharacterMotion3d",
        text: "Copied historical scene-owned 3D character result. Gameplay chooses gravity, jumping and recovery. This is not a live contact or pending request.",
        members: &[
            value(
                "translation",
                "Total world-space Vec3 displacement applied once, including actual platform carry.",
            ),
            value(
                "movement",
                "CharacterMovement3d from the carried pose, excluding carry translation and hits.",
            ),
            value(
                "grounded",
                "Classified walkable final support without penetration or ascending input; false when ground is now inactive or removed.",
            ),
            value(
                "ground",
                "Zero-travel endpoint support RayHit3d or null when absent/inactive. It is independent of raw backend grounded.",
            ),
            value(
                "ground_walkable",
                "Whether the active ground hit satisfies authored up and slope limit.",
            ),
            value(
                "ground_started_penetrating",
                "Whether the endpoint support probe started in strict penetration.",
            ),
            value(
                "platform",
                "CharacterCarry3d or null without verified carry or when its entity is now inactive/removed.",
            ),
        ],
    },
    TypeEntry {
        name: "CharacterMovement3d",
        text: "Copied raw Rapier movement phase. Backend flags are not classified walkable support and must not alone authorize snapping. No independent step/snap displacement or iteration-limit status is available.",
        members: &[
            value(
                "translation",
                "Collision-limited world-space Vec3 displacement of this phase.",
            ),
            value(
                "grounded",
                "Raw backend nearby upward-facing contact flag; may include steep or predicted contacts.",
            ),
            value(
                "sliding_down_slope",
                "Raw backend slope-handling flag, which can also be set while climbing.",
            ),
            value(
                "collisions",
                "Ordered copied CharacterCollision3d list, filtered to active existing hit entities.",
            ),
        ],
    },
    TypeEntry {
        name: "CharacterCollision3d",
        text: "One historical movement-phase collision; internal stair and snap probes are not events.",
        members: &[
            value(
                "hit",
                "Copied world-space RayHit3d for the collided piece; distance belongs to this sweep phase.",
            ),
            value(
                "translation_applied",
                "World-space Vec3 movement already applied at this collision.",
            ),
            value(
                "translation_remaining",
                "World-space Vec3 movement remaining at this collision.",
            ),
        ],
    },
    TypeEntry {
        name: "CharacterCarry3d",
        text: "Verified solved platform carry, already included in total character translation. A platform rotation moves the probe origin along a straight swept chord; it does not rotate the actor.",
        members: &[
            value(
                "entity",
                "Active existing platform entity whose solved movement carried the actor.",
            ),
            value(
                "requested",
                "World-space Vec3 displacement requested by the platform's previous/current poses.",
            ),
            value(
                "motion",
                "Collision-limited CharacterMovement3d carry phase, separate from ordinary movement.",
            ),
        ],
    },
];
