//! The selection's outline: the design draws the selected object with a thin
//! bright-metal edge around its own shape, not a box.
//!
//! The engine already has what that takes. A sprite's colour transform can
//! paint every pixel it draws one solid colour (multiply by nothing, offset by
//! the colour), so a copy of the selected sprite painted bright metal, nudged a
//! pixel and a half each way and drawn just behind the sprite itself, leaves
//! exactly an edge around its silhouette. Nothing in the renderer is special to
//! the editor; the copies live only in a scratch world the canvas draws, never
//! in the scene being edited or its file.

use serde_json::{Value, json};
use sindri_core::{EntityData, EntityId, Transform3D, World};

/// The design's bright metal, `#FFBE55`, as the colour transform's offset.
/// In linear light, because that is what a sprite's colour is worked in: the
/// sRGB target turns it back into `#FFBE55` on the screen.
const METAL_BRIGHT: [f32; 4] = [1.0, 0.5149, 0.0908, 0.0];

/// How far behind the sprite the copies sit. Sprites on one layer are painted
/// back to front by depth, so this puts the edge under the sprite and over
/// whatever was under it.
const BEHIND: f32 = 1.0e-4;

const SPRITE: &str = "sindri.sprite";

/// `world` with an outline drawn around `entity`'s sprite, `width` world units
/// wide, or `None` when it has no sprite drawn to outline.
pub fn outlined(world: &World, entity: EntityId, width: f32) -> Option<World> {
    if !world.is_active(entity) || !width.is_finite() || width <= 0.0 {
        return None;
    }
    let sprite = world.get(entity)?.components.get(SPRITE)?;
    let alpha = sprite
        .get("tint")
        .and_then(|tint| tint.get(3))
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    if alpha <= 0.0 {
        return None;
    }
    let transform = world.world_transform(entity)?;
    let edge = edge_sprite(sprite, alpha);
    let mut outlined = world.clone();
    // Eight directions rather than four, so a diagonal edge is as thick as a
    // straight one.
    for (dx, dy) in [
        (1.0, 0.0),
        (-1.0, 0.0),
        (0.0, 1.0),
        (0.0, -1.0),
        (0.7, 0.7),
        (-0.7, 0.7),
        (0.7, -0.7),
        (-0.7, -0.7),
    ] {
        let [x, y, z] = transform.position;
        outlined.spawn(EntityData {
            transform_3d: Some(Transform3D {
                position: [x + dx * width, y + dy * width, z - BEHIND],
                ..transform
            }),
            components: [(SPRITE.to_owned(), edge.clone())].into(),
            ..EntityData::default()
        });
    }
    Some(outlined)
}

/// The selected sprite as one flat colour: the same picture, layer and
/// drawing, unlit so the edge shows in a dark scene, with no shader or normal
/// map to change its shape or shade.
fn edge_sprite(sprite: &Value, alpha: f64) -> Value {
    let mut edge = sprite.clone();
    if let Some(fields) = edge.as_object_mut() {
        for removed in ["shader", "shader_params", "normal_map"] {
            fields.remove(removed);
        }
        fields.insert("tint".to_owned(), json!([1.0, 1.0, 1.0, alpha]));
        fields.insert(
            "color_transform".to_owned(),
            json!({ "multiply": [0.0, 0.0, 0.0, 1.0], "offset": METAL_BRIGHT }),
        );
        fields.insert("unlit".to_owned(), json!(true));
    }
    edge
}
