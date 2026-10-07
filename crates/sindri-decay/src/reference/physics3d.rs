//! Prose for the explicitly dimensioned 3D surface.

use super::{TypeEntry, call, value};

pub(super) const TYPES: &[TypeEntry] = &[
    TypeEntry {
        name: "Physics3d",
        text: "Independent 3D physics controls, indexed queries and copied last-step events. Requires a 3D physics host. Live controls require an active synchronized body; calls before spawn synchronization fail rather than queue. Authored motion fields are unchanged, so structural rebuilds restore authored settings.",
        members: &[
            call(
                "layer",
                &["name"],
                "Query mask bit for a text name in the active authored 3D world's first 32 layer labels. Empty labels cannot be selected; duplicate names select the first bit. Reads current authoring without stepping; ignores inactive settings and never reads 2D names. Unknown names, malformed or multiple active world settings and missing 3D host fail.",
            ),
            call(
                "mask",
                &["names"],
                "Combines Physics3d.layer bits from a List<str> using bitwise OR. Repeated names do not add bits; an empty list returns zero. Every name is checked; non-text elements, unknown names, invalid active world settings or missing 3D host fail.",
            ),
            call(
                "raycast",
                &[
                    "origin",
                    "direction",
                    "max_distance",
                    "mask",
                    "include_sensors",
                    "exclude",
                ],
                "Closest indexed 3D ray hit as a copied RayHit3d or null. Finite Vec3 origin/direction and non-negative finite f32-range distance are required; direction is normalized and the endpoint must remain finite. Whole-u32 mask selects memberships, sensors opt in and an Entity or null excludes all pieces of one entity. Skips inactive/despawned entities. Inside/on hits have distance zero and zero normal; exact ties prefer entity then piece order. Queries synchronized geometry without stepping physics.",
            ),
            call(
                "overlap_sphere",
                &["centre", "radius", "mask", "include_sensors", "exclude"],
                "Sorted unique copied Entity list overlapping a finite Vec3 centre and positive finite f32-range radius. Uses indexed synchronized geometry, membership mask, sensor opt-in and whole-entity exclusion; inactive/despawned entities are filtered. Does not step physics.",
            ),
            call(
                "cast_sphere",
                &[
                    "origin",
                    "radius",
                    "direction",
                    "max_distance",
                    "mask",
                    "include_sensors",
                    "exclude",
                ],
                "Closest fixed-orientation sphere sweep as a copied RayHit3d or null. Positive finite radius, finite Vec3 origin/direction, normalized nonzero direction and non-negative distance in engine f32 range; endpoint must remain finite. Point is world contact, distance is probe travel. Initial overlap returns origin, zero distance and zero normal. Uses indexed synchronized geometry and the ray filter, skipping inactive/despawned entities. No rotation or physics step; extreme finite geometry still has the engine's documented numerical limitations.",
            ),
            call(
                "overlap_box",
                &[
                    "centre",
                    "half_extents",
                    "rotation_axis",
                    "rotation_angle",
                    "mask",
                    "include_sensors",
                    "exclude",
                ],
                "Indexed active 3D shape query. Positive Vec3 half-extents define the box. Finite f32-range values are required. A nonzero Vec3 rotation_axis is normalized and rotation_angle is radians about it using the right-hand rule. Uses the ray membership mask, sensor opt-in and whole-entity exclusion. Overlaps are sorted unique copied Entity lists; casts retain initial-overlap and deterministic tie semantics. Does not step physics.",
            ),
            call(
                "cast_box",
                &[
                    "origin",
                    "half_extents",
                    "rotation_axis",
                    "rotation_angle",
                    "direction",
                    "max_distance",
                    "mask",
                    "include_sensors",
                    "exclude",
                ],
                "Indexed active 3D shape query. Positive Vec3 half-extents define the box. Sweeps return copied RayHit3d or null with fixed orientation and nonzero normalized direction; travel is non-negative and the endpoint finite. Finite f32-range values are required. A nonzero Vec3 rotation_axis is normalized and rotation_angle is radians about it using the right-hand rule. Uses the ray membership mask, sensor opt-in and whole-entity exclusion. Overlaps are sorted unique copied Entity lists; casts retain initial-overlap and deterministic tie semantics. Does not step physics.",
            ),
            call(
                "overlap_capsule",
                &[
                    "centre",
                    "half_height",
                    "radius",
                    "rotation_axis",
                    "rotation_angle",
                    "mask",
                    "include_sensors",
                    "exclude",
                ],
                "Indexed active 3D shape query. The capsule lies along local Y before rotation; half_height is the non-negative half-length of its straight segment and radius is positive. Zero half-height is a sphere. Finite f32-range values are required. A nonzero Vec3 rotation_axis is normalized and rotation_angle is radians about it using the right-hand rule. Uses the ray membership mask, sensor opt-in and whole-entity exclusion. Overlaps are sorted unique copied Entity lists; casts retain initial-overlap and deterministic tie semantics. Does not step physics.",
            ),
            call(
                "cast_capsule",
                &[
                    "origin",
                    "half_height",
                    "radius",
                    "rotation_axis",
                    "rotation_angle",
                    "direction",
                    "max_distance",
                    "mask",
                    "include_sensors",
                    "exclude",
                ],
                "Indexed active 3D shape query. The local-Y capsule has non-negative straight-segment half_height and positive radius; zero half-height is a sphere. Sweeps return copied RayHit3d or null with fixed orientation and nonzero normalized direction; travel is non-negative and the endpoint finite. Finite f32-range values are required. A nonzero Vec3 rotation_axis is normalized and rotation_angle is radians about it using the right-hand rule. Uses the ray membership mask, sensor opt-in and whole-entity exclusion. Overlaps are sorted unique copied Entity lists; casts retain initial-overlap and deterministic tie semantics. Does not step physics.",
            ),
            call(
                "velocity",
                &["entity"],
                "A copied world-space Vec3 linear velocity of an active synchronized 3D body.",
            ),
            call(
                "set_velocity",
                &["entity", "velocity"],
                "Sets finite world-space Vec3 linear velocity on an active synchronized dynamic or velocity-kinematic body. Invalid inputs fail before mutation; does not change authored starting motion.",
            ),
            call(
                "angular_velocity",
                &["entity"],
                "A copied world-space Vec3 angular velocity in radians per second around XYZ of an active synchronized 3D body.",
            ),
            call(
                "set_angular_velocity",
                &["entity", "velocity"],
                "Sets finite Vec3 angular velocity in radians per second around world XYZ on an active synchronized dynamic or velocity-kinematic body. Rotation-locked bodies retain zero angular velocity. Invalid input fails before mutation; does not change authored starting motion.",
            ),
            call(
                "apply_impulse",
                &["entity", "impulse"],
                "Applies a finite world-space Vec3 impulse to an active synchronized dynamic body using its compound mass. Invalid input or body kind fails before mutation; does not change authored starting motion.",
            ),
            call(
                "collision_started",
                &[],
                "Sorted unique other active entities that started solid contact with this script's entity in the last successful 3D step. All scripts observe the same snapshot without draining it.",
            ),
            call(
                "collision_stopped",
                &[],
                "Sorted unique other active entities that stopped solid contact with this script's entity in the last successful 3D step. Removed or inactive handles are filtered.",
            ),
            call(
                "sensor_entered",
                &[],
                "Sorted unique other active entities that entered sensor contact with this script's entity in the last successful 3D step. Discrete events do not guarantee fast trigger sweeps; the snapshot is not drained.",
            ),
            call(
                "sensor_exited",
                &[],
                "Sorted unique other active entities that left sensor contact with this script's entity in the last successful 3D step. Removed or inactive handles are filtered; the snapshot is not drained.",
            ),
        ],
    },
    TypeEntry {
        name: "RayHit3d",
        text: "Copied closest 3D ray/shape-cast hit, optional where no hit exists. World-space point/normal and world-unit distance; editing fields never changes simulation geometry.",
        members: &[
            value("entity", "The active entity whose collider piece was hit."),
            value(
                "point",
                "World-space Vec3 hit/contact position; probe origin for an initial shape overlap.",
            ),
            value(
                "normal",
                "World-space outward Vec3 normal; zero for an inside/on or initial-overlap hit.",
            ),
            value(
                "distance",
                "World-unit ray distance or shape centre travel, including zero for initial overlap.",
            ),
        ],
    },
];
