//! Camera component schemas and editor field meanings.

use super::SceneExtractError;
use crate::{CameraBehaviorComponent, CameraComponent, CameraOrbitComponent};
use sindri_core::{ComponentSchemaRegistry, FieldMeaning};

pub(super) fn register_cameras(
    components: &mut ComponentSchemaRegistry,
) -> Result<(), SceneExtractError> {
    components.register_with_default::<CameraComponent>(
        "Camera",
        serde_json::json!({
            "projection": CameraComponent::PROJECTIONS[0],
            "vertical_fov_degrees": CameraComponent::DEFAULT_VERTICAL_FOV_DEGREES,
            "near": CameraComponent::DEFAULT_NEAR,
            "far": CameraComponent::DEFAULT_FAR
        }),
    )?;
    components.register_with_default::<CameraBehaviorComponent>(
        "Camera Behavior",
        serde_json::json!({
            "follow": null,
            "confine": null,
            "shake": {
                "trauma": 0.0,
                "strength": 0.12,
                "decay": 2.8,
                "frequency": 57.0,
                "phase": 0.0
            }
        }),
    )?;
    describe_camera_behavior(components)?;
    components.register_with_fields::<CameraOrbitComponent>(
        "Camera Orbit",
        serde_json::json!({
            "target": "target", "offset": [0.0, 0.0, 0.0], "yaw": 0.0, "pitch": 0.35,
            "distance": 6.0, "smoothing": 8.0, "collision_mask": u32::MAX,
            "collision_padding": 0.2
        }),
    )?;
    components.describe::<CameraOrbitComponent>([
        ("target", FieldMeaning::Entity),
        ("yaw", FieldMeaning::Angle),
        ("pitch", FieldMeaning::Angle),
        (
            "distance",
            FieldMeaning::Range {
                min: 0.0,
                max: f64::INFINITY,
            },
        ),
        (
            "smoothing",
            FieldMeaning::Range {
                min: 0.0,
                max: f64::INFINITY,
            },
        ),
        (
            "collision_mask",
            FieldMeaning::Range {
                min: 0.0,
                max: f64::from(u32::MAX),
            },
        ),
        (
            "collision_padding",
            FieldMeaning::Range {
                min: 0.0,
                max: f64::INFINITY,
            },
        ),
    ])?;
    Ok(())
}

/// A camera's behaviour follows and confines only once those are added, so
/// what each holds is said here, and then what its fields mean: the target is
/// an entity in the scene, and the rates are rates. The shake's phase and
/// offset are the shake's own running state rather than anything to tune.
fn describe_camera_behavior(
    components: &mut ComponentSchemaRegistry,
) -> Result<(), SceneExtractError> {
    let at_least_zero = || FieldMeaning::Range {
        min: 0.0,
        max: f64::INFINITY,
    };
    components.describe_optional::<CameraBehaviorComponent>(
        "follow",
        serde_json::json!({
            "target": "", "offset": [0.0, 0.0, 0.0], "dead_zone": [0.0, 0.0],
            "smoothing": 8.0, "max_speed": 0.0
        }),
    )?;
    components.describe_optional::<CameraBehaviorComponent>(
        "confine",
        serde_json::json!({ "min": [-10.0, -10.0], "max": [10.0, 10.0] }),
    )?;
    components.describe::<CameraBehaviorComponent>([
        ("follow.target", FieldMeaning::Entity),
        ("follow.smoothing", at_least_zero()),
        ("follow.max_speed", at_least_zero()),
        ("shake.trauma", FieldMeaning::Range { min: 0.0, max: 1.0 }),
        ("shake.strength", at_least_zero()),
        ("shake.decay", at_least_zero()),
        ("shake.frequency", at_least_zero()),
    ])?;
    Ok(())
}
