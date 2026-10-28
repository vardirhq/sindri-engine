//! Prose for the questions a script can ask the 2D lighting.

use super::{TypeEntry, call};

pub(super) const TYPES: &[TypeEntry] = &[TypeEntry {
    name: "Lighting",
    text: "Asks the scene's 2D lighting about places in the world. Requires a scene whose environment has 2D lighting on, and fails otherwise. Every light and occluder in the world counts, wherever the camera is. A point is lit by a light when no occluder edge crosses the line between it and the light's centre, so a soft penumbra is a hard edge here, and a light that does not cast shadows shines through occluders.",
    members: &[
        call(
            "visible",
            &["from", "to"],
            "Whether the straight line between two world points (Vec2) crosses no occluder edge: line of sight. Touching an edge's end or standing on it does not block.",
        ),
        call(
            "level",
            &["point"],
            "How bright it is at a world point (Vec2): the luminance of the ambient plus every light reaching it, shadows applied. 0 is dark, 1 is a white light at full strength, and overlapping lights add past 1.",
        ),
    ],
}];
