//! Where things are and how they are turned: quaternions and transforms.

use super::{TypeEntry, call, value};

pub(super) const TYPES: &[TypeEntry] = &[
    TypeEntry {
        name: "Quat",
        text: "A copied quaternion value, with x/y/z/w components. Build with Quat(x: 0.0, y: 0.0, z: 0.0, w: 1.0), or copy transform.rotation. Change a local var's components, then assign the whole value; no quaternion arithmetic or interpolation methods are provided.",
        members: &[
            value("x", "The quaternion's X component."),
            value("y", "The quaternion's Y component."),
            value("z", "The quaternion's Z component."),
            value(
                "w",
                "The quaternion's scalar component; identity has w = 1 and x/y/z = 0.",
            ),
        ],
    },
    TypeEntry {
        name: "Transform",
        text: "Where an object is, how it is turned and how big it is. For an object attached to another, these are measured from that parent.",
        members: &[
            value(
                "rotation",
                "Parent-relative Quat in x/y/z/w order. Reads copy the stored rotation; whole writes normalize finite nonzero values atomically and preserve position, scale and Z lock. Stored components are readable; change components on a local var and assign it back. Zero/non-finite writes fail without mutation.",
            ),
            value(
                "forward",
                "Read-only unit world-space facing direction: -Z at identity. Parent rotation is included and scale is ignored. Copy it to move with `world_position += forward * speed * dt`.",
            ),
            value(
                "right",
                "Read-only unit world-space +X direction, with parent rotation included and scale ignored.",
            ),
            value(
                "up",
                "Read-only unit world-space +Y direction, with parent rotation included and scale ignored.",
            ),
            call(
                "look_at",
                &["target"],
                "Faces a world-space Vec3 target with Y up and zero world roll; preserves position and scale, including under a parent. Vertical targets use yaw zero. Coincident/non-finite targets are errors with no mutation.",
            ),
            call(
                "to_local",
                &["point"],
                "A world-space Vec3 point measured in this object's own space: its position subtracted, its world rotation undone and its world scale divided out. What a point on a moving, turning ship is in the ship's own blocks. Reads only; zero scale is an error.",
            ),
            call(
                "to_world",
                &["point"],
                "A Vec3 point in this object's own space placed in the world: scaled, turned and moved by the object's world transform, parents included. The inverse of `to_local`. Reads only.",
            ),
            call(
                "rotate_around",
                &["pivot", "axis", "radians"],
                "Rotates position and orientation around a world-space Vec3 pivot and nonzero Vec3 axis by radians. Normalizes the axis and preserves scale. Invalid input/output, a singular parent scale, or movement off a Z-locked layer is an error with no mutation.",
            ),
            value(
                "position",
                "Where the object is. Change it to move the object: `this.transform.position.x += speed * dt`. Measured from the parent if it has one.",
            ),
            value(
                "pitch",
                "How far a 3D object is tipped up (positive) or down from level, in radians, after its yaw. Changing it keeps its yaw and roll.",
            ),
            value(
                "roll",
                "How far a 3D object is turned about the way it faces, in radians, after its yaw and pitch. Changing it keeps its yaw and pitch.",
            ),
            value(
                "rotation_z",
                "How far the object is turned, in radians. A full turn is `TAU`. This is the turn of a flat object in a 2D game; it replaces any 3D turn.",
            ),
            value(
                "scale",
                "How big the object is on each axis; 1 is its normal size.",
            ),
            value(
                "world_position",
                "Where the object is in the world, even when it is attached to a parent. Use it to compare positions of objects in different places.",
            ),
            value(
                "yaw",
                "Which way a 3D object faces on the ground, in radians: a turn about the up (Y) axis. At zero it faces -Z, and positive turns it left. It moves along `Vec3(-sin(yaw), 0.0, -cos(yaw))`, and faces a point `d` away with `atan2(-d.x, -d.z)`. Changing it keeps its pitch and roll.",
            ),
        ],
    },
];
