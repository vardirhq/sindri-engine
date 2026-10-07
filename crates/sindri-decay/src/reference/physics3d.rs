//! Prose for the explicitly dimensioned 3D surface.

use super::{TypeEntry, call};

pub(super) const TYPES: &[TypeEntry] = &[TypeEntry {
    name: "Physics3d",
    text: "Independent 3D physics controls and copied last-step events. Requires a 3D physics host. Live controls require an active synchronized body; calls before spawn synchronization fail rather than queue. Authored motion fields are unchanged, so structural rebuilds restore authored settings.",
    members: &[
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
}];
