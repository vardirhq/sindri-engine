//! Runtime controls over the authored scene environment.
use super::{TypeEntry, value};

pub(super) const TYPES: &[TypeEntry] = &[
    TypeEntry {
        name: "SceneEnvironment",
        text: "The existing authored scene environment on an entity. Changes affect the next extracted frame without restarting. Missing components and stale entities are runtime errors; writes validate the full payload atomically. Fog, the color grade and bloom are reached through `fog`, `post_process` and `bloom`.",
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
            value("fog", "The distance and height fog, as `SceneFog`."),
            value(
                "post_process",
                "The color grade applied to the lit world, as `ScenePostProcess`.",
            ),
            value("bloom", "The glow around bright light, as `SceneBloom`."),
        ],
    },
    TypeEntry {
        name: "SceneFog",
        text: "Fog over the 3D world, from the camera outward and thickest low down. A section the scene never authored starts from its defaults, and a write keeps every other field. Refused values leave the payload as it was.",
        members: &[
            value("enabled", "Whether the fog is drawn at all."),
            value(
                "color",
                "What the fog fades towards, as a Color with alpha one.",
            ),
            value("start", "World distance where fog begins; not negative."),
            value(
                "distance",
                "World distance over which it thickens to full; greater than 0.",
            ),
            value(
                "density",
                "Extra exponential fog, 0 to 1. Zero leaves only the distance term.",
            ),
            value("height", "World height below which extra fog gathers."),
            value(
                "height_falloff",
                "How strongly fog gathers below `height`, 0 to 1.",
            ),
        ],
    },
    TypeEntry {
        name: "ScenePostProcess",
        text: "The grade applied to the lit world before the interface is drawn over it.",
        members: &[
            value(
                "exposure",
                "Brightness in photographic stops, -8 to 8; one stop doubles it.",
            ),
            value(
                "contrast",
                "Contrast about mid-grey, 0 to 4; 1 leaves it alone.",
            ),
            value(
                "saturation",
                "Color strength, 0 to 4; 0 is grey and 1 leaves it alone.",
            ),
            value("vignette", "How much the edges darken, 0 to 1."),
        ],
    },
    TypeEntry {
        name: "SceneBloom",
        text: "The glow that spreads from bright light, such as a lamp or an emissive model.",
        members: &[
            value("enabled", "Whether anything glows."),
            value(
                "threshold",
                "How bright a pixel must be to glow; not negative.",
            ),
            value(
                "knee",
                "How softly glow fades in around the threshold, 0.0001 to 1.",
            ),
            value("intensity", "How strong the glow is; not negative."),
        ],
    },
];
