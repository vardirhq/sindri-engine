//! Runtime controls over the authored scene environment.
use super::{TypeEntry, value};

pub(super) const TYPES: &[TypeEntry] = &[TypeEntry {
    name: "SceneEnvironment",
    text: "The existing authored scene environment on an entity. Changes affect the next extracted frame without restarting. Missing components and stale entities are runtime errors; writes validate the full payload atomically. Fog and post-processing controls are not exposed here.",
    members: &[
        value(
            "ambient_intensity",
            "Finite nonnegative ambient strength. Fractional writes preserve their value even when the authored JSON number was an integer.",
        ),
        value(
            "ambient_color",
            "Ambient RGB as a Color with alpha one. Whole and channel writes are checked; alpha must remain one. RGB follows the authored environment contract: finite f32 values.",
        ),
        value(
            "background",
            "The scene clear color as RGBA Color. Whole and channel writes retain finite f32 values, including authored alpha. Failed writes preserve the old payload.",
        ),
    ],
}];
