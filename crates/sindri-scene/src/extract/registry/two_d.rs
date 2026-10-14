//! The 2D drawables whose blanks are long enough to be their own file.

use sindri_core::{AssetKind, ComponentSchemaRegistry, FieldMeaning};

use crate::components::{
    Light2dComponent, Light2dOccluderComponent, OccluderShape, ShapeComponent, UiShapeBlend,
    UiShapeComponent, UiShapeKind,
};

use crate::effects::{EmissionShape, ParticleSpace, ParticlesComponent};
use crate::textures::PROCEDURAL_TEXTURES;

use super::super::SceneExtractError;

/// The two shape components, whose blanks are long enough to be their own
/// function.
///
/// Long because a shape is described entirely by numbers — there is no texture
/// standing in for the look — and because a blank that drew nothing would read
/// as a broken add rather than an empty one. Both default to a mint outline.
pub(super) fn register_shapes(
    components: &mut ComponentSchemaRegistry,
) -> Result<(), SceneExtractError> {
    // The same blank as the overlay shape, minus the anchor: a mint outline,
    // because a shape whose default drew nothing would read as a broken add.
    components.register_with_default::<ShapeComponent>(
        "Shape",
        serde_json::json!({
            "kind": UiShapeKind::default().as_str(),
            "count": 6.0,
            "fill": [0.0, 0.0, 0.0, 0.0],
            "stroke": [0.49, 1.0, 0.77, 1.0],
            "stroke_width": 0.06,
            "corner_radius": 0.0,
            "dashes": 0.0,
            "dash_duty": 0.5,
            "sweep_start": 0.0,
            "sweep_turns": 1.0,
            "blend": UiShapeBlend::default().as_str(),
            "layer": 0
        }),
    )?;
    components.register_with_default::<UiShapeComponent>(
        "UI Shape",
        serde_json::json!({
            "kind": UiShapeKind::default().as_str(),
            "count": 6.0,
            "fill": [0.0, 0.0, 0.0, 0.0],
            "stroke": [0.49, 1.0, 0.77, 1.0],
            "stroke_width": 0.04,
            "corner_radius": 0.0,
            "dashes": 0.0,
            "dash_duty": 0.5,
            "sweep_start": 0.0,
            "sweep_turns": 1.0,
            "blend": UiShapeBlend::default().as_str(),
            "anchor": "center",
            "layer": 0,
            "shadow": {
                "color": [0.0, 0.0, 0.0, 0.0],
                "offset": [0.0, 0.0],
                "blur": 0.0,
                "spread": 0.0
            }
        }),
    )?;
    Ok(())
}

/// 2D lights and occluders. A fresh light is a warm lamp four units across; a
/// fresh occluder is a unit box.
pub(super) fn register_lights(
    components: &mut ComponentSchemaRegistry,
) -> Result<(), SceneExtractError> {
    components.register_with_default::<Light2dComponent>(
        "Light 2D",
        serde_json::to_value(Light2dComponent::default()).expect("a 2D light serializes"),
    )?;
    components.register_with_default::<Light2dOccluderComponent>(
        "Light Occluder 2D",
        serde_json::to_value(Light2dOccluderComponent::default()).expect("an occluder serializes"),
    )?;
    let at_least = |min: f64| FieldMeaning::Range {
        min,
        max: f64::INFINITY,
    };
    components.describe::<Light2dComponent>([
        ("color", FieldMeaning::Color),
        ("intensity", at_least(0.0)),
        ("radius", at_least(0.01)),
        ("falloff", at_least(0.01)),
        ("height", at_least(0.0)),
        ("shadow_reach", at_least(0.0)),
    ])?;
    components.describe::<Light2dOccluderComponent>([
        ("shape", FieldMeaning::choice(OccluderShape::NAMES)),
        ("size", at_least(0.01)),
    ])?;
    Ok(())
}

/// A particle emitter. A fresh one is a slow fountain of the checkerboard, so
/// adding it shows something moving rather than a component that seems inert.
pub(super) fn register_particles(
    components: &mut ComponentSchemaRegistry,
) -> Result<(), SceneExtractError> {
    let blank = ParticlesComponent {
        texture: PROCEDURAL_TEXTURES[0].reference.to_owned(),
        ..ParticlesComponent::default()
    };
    components.register_with_default::<ParticlesComponent>(
        "Particles",
        serde_json::to_value(blank).expect("an emitter serializes"),
    )?;
    let at_least = |min: f64| FieldMeaning::Range {
        min,
        max: f64::INFINITY,
    };
    components.describe::<ParticlesComponent>([
        ("texture", FieldMeaning::Asset(AssetKind::Texture)),
        ("rate", at_least(0.0)),
        ("lifetime", at_least(0.01)),
        ("speed", at_least(0.0)),
        ("drag", at_least(0.0)),
        ("size", at_least(0.0)),
        ("size_end", at_least(0.0)),
        ("shape", FieldMeaning::choice(EmissionShape::NAMES)),
        ("space", FieldMeaning::choice(ParticleSpace::NAMES)),
    ])?;
    Ok(())
}
