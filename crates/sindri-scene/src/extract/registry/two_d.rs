//! The 2D drawables whose blanks are long enough to be their own file.

use sindri_core::{AssetKind, ComponentSchemaRegistry, FieldMeaning};

use crate::components::{
    Light2dComponent, Light2dOccluderComponent, OccluderShape, ShapeComponent, SpriteComponent,
    UiShapeBlend, UiShapeComponent, UiShapeKind,
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

/// A sprite mask. A fresh one is the checkerboard's square, so a sprite set
/// to show inside it shows a square at once.
pub(super) fn register_sprite_mask(
    components: &mut ComponentSchemaRegistry,
) -> Result<(), SceneExtractError> {
    components.register_with_default::<crate::SpriteMaskComponent>(
        "Sprite Mask",
        serde_json::json!({ "texture": PROCEDURAL_TEXTURES[0].reference, "name": "" }),
    )?;
    components.describe::<crate::SpriteMaskComponent>([(
        "texture",
        FieldMeaning::Asset(AssetKind::Texture),
    )])?;
    Ok(())
}

/// `sindri.text`: the UI label's fields, in world units. A fresh one is half a
/// unit tall, which reads beside a one-unit sprite, and centred on its entity.
pub(super) fn register_world_text(
    components: &mut ComponentSchemaRegistry,
) -> Result<(), SceneExtractError> {
    components.register_with_fields::<crate::TextComponent>(
        "Text",
        serde_json::json!({
            "text": "Text",
            "font": "",
            "font_size": 0.5,
            "line_height": 0.6,
            "color": [1.0, 1.0, 1.0, 1.0],
            "anchor": "center",
            "layer": 0,
            "values": [],
            "bounds": [0.0, 0.0],
            "wrap": "none",
            "line_align": "follow",
            "letter_spacing": 0.0,
            "bold": false,
            "italic": false,
            "case": "as_written",
            "visible": -1.0,
            "outline": { "width": 0.0, "color": [0.0, 0.0, 0.0, 1.0] },
            "shadow": {
                "offset": [0.0, 0.0],
                "color": [0.0, 0.0, 0.0, 1.0],
                "softness": 0.0
            },
            "auto_size": { "enabled": false, "min": 0.1 },
            "rich": false,
            "follow_rotation": false
        }),
    )?;
    Ok(())
}

pub(super) fn register_sprite(
    components: &mut ComponentSchemaRegistry,
) -> Result<(), SceneExtractError> {
    components.register_with_default::<SpriteComponent>(
        "Sprite",
        serde_json::json!({
            "texture": PROCEDURAL_TEXTURES[0].reference,
            "tint": [1.0, 1.0, 1.0, 1.0],
            "color_transform": {
                "multiply": [1.0, 1.0, 1.0, 1.0],
                "offset": [0.0, 0.0, 0.0, 0.0]
            },
            "layer": 0,
            // Off by default: a new sprite is as likely to be something
            // lying on the ground as somebody standing on it.
            "billboard": false,
            "shader": "",
            "shader_params": [0.0, 0.0, 0.0, 0.0],
            "normal_map": "",
            "unlit": false,
            "draw": "simple", "border_scale": 1.0 / 16.0, "mask": "none", "mask_name": ""
        }),
    )?;
    Ok(())
}
