//! Managed gameplay tweens; Weave owns its CSS syntax.
use super::{TypeEntry, call};
pub(super) const TYPES: &[TypeEntry] = &[
    TypeEntry {
        name: "Tween",
        text: "Creates automatically playing gameplay tweens owned by the calling script. Copies alias playback state. Advance occurs before the owner's subsequent updates using script delta. No automatic property binding. Curves: linear, ease, ease-in, ease-out, ease-in-out. Finite endpoints and finite non-negative duration are required; zero duration completes immediately. Maximum 8192 retained handles per runner; dispose unused handles.",
        members: &[
            call(
                "number",
                &["from", "to", "duration", "easing"],
                "Starts a NumberTween; duration is seconds.",
            ),
            call(
                "vec2",
                &["from", "to", "duration", "easing"],
                "Starts a Vec2Tween in the endpoints' coordinate space.",
            ),
            call(
                "vec3",
                &["from", "to", "duration", "easing"],
                "Starts a Vec3Tween in the endpoints' coordinate space.",
            ),
            call(
                "color",
                &["from", "to", "duration", "easing"],
                "Starts a ColorTween, blending all four channels without additional gamma conversion or channel clamping.",
            ),
            call(
                "number_value",
                &["tween"],
                "Reads the eased number, held at the exact target after completion.",
            ),
            call("vec2_value", &["tween"], "Reads the eased Vec2."),
            call(
                "vec3_value",
                &["tween"],
                "Reads the eased Vec3. Apply through ordinary checked transform setters.",
            ),
            call(
                "color_value",
                &["tween"],
                "Reads the eased Color including alpha.",
            ),
            call(
                "progress",
                &["tween"],
                "Linear elapsed fraction from 0 to 1; zero duration is 1.",
            ),
            call(
                "is_done",
                &["tween"],
                "True on natural completion, including zero duration. Cancellation is not completion.",
            ),
            call("is_paused", &["tween"], "Whether advancement is paused."),
            call(
                "is_cancelled",
                &["tween"],
                "Whether cancelled; its last value remains readable until disposed.",
            ),
            call("pause", &["tween"], "Pauses at the current value."),
            call(
                "resume",
                &["tween"],
                "Resumes a paused tween. Does not undo cancellation.",
            ),
            call(
                "cancel",
                &["tween"],
                "Stops at the current value; is_done is false. Restart can play it again.",
            ),
            call(
                "restart",
                &["tween"],
                "Resets to the original endpoints and begins again, clearing pause/cancellation.",
            ),
            call(
                "dispose",
                &["tween"],
                "Releases a tween; aliases become invalid. Owner removal, script replacement and runner reset also release it.",
            ),
        ],
    },
    TypeEntry {
        name: "TweenHandle",
        text: "The common handle accepted by playback controls; only factories construct one.",
        members: &[],
    },
    TypeEntry {
        name: "NumberTween",
        text: "A managed number tween; read with Tween.number_value.",
        members: &[],
    },
    TypeEntry {
        name: "Vec2Tween",
        text: "A managed Vec2 tween; read with Tween.vec2_value.",
        members: &[],
    },
    TypeEntry {
        name: "Vec3Tween",
        text: "A managed Vec3 tween; read with Tween.vec3_value.",
        members: &[],
    },
    TypeEntry {
        name: "ColorTween",
        text: "A managed Color tween; read with Tween.color_value.",
        members: &[],
    },
];
