//! What the app's inspector is drawn from: the selection as the engine holds
//! it, with everything the app needs to present it the design's way.
//!
//! The app turns this into friendly sections (Place in tiles, Light 2D with
//! sliders and colour swatches, Sprite, Collider 2D …), so it needs more than
//! the stored payload: every field at its effective value, defaults included,
//! and what the engine says each field means (a range, a choice, a colour).
//! Both come from the component registry, the same source the desktop
//! inspector uses, so the phone never guesses a bound the engine would refuse.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};
use sindri_core::{ComponentSchemaRegistry, FieldMeaning, SpriteRef, SpriteSheetDocument, World};
use sindri_scene::{SceneExtractor, TilemapComponent};

use crate::Editing;

impl Editing {
    /// The selection for the inspector, as JSON, or `null` with nothing
    /// selected:
    ///
    /// - `name`, `id` (its stable scene ID, if any);
    /// - `position`, `turn` (degrees about the view axis, clockwise as seen)
    ///   and `scale`, from its transform;
    /// - `tile`: how many world units a tile of the scene's tilemap is, which
    ///   the app shows places and reaches in;
    /// - `components`: each one's `type`, `display` name, `values` (every
    ///   field, defaults filled in), `fields` (its top-level numbers and
    ///   switches in the registry's order, for components the app has no
    ///   presentation of) and `meanings` (field path to what it means);
    /// - `sprite`: for a sprite, its `texture`, its `region` in the sheet and
    ///   the `rect` that region covers, as fractions of the image (left, top,
    ///   width, height), from the project's `sheets`, for a thumbnail.
    pub fn inspector(
        &self,
        world: &World,
        extractor: &SceneExtractor,
        sheets: &BTreeMap<String, SpriteSheetDocument>,
    ) -> Value {
        let Some(entity) = self.selected else {
            return Value::Null;
        };
        let Some(data) = world.get(entity) else {
            return Value::Null;
        };
        let registry = extractor.components();
        let components: Vec<Value> = data
            .components
            .iter()
            .map(|(type_name, stored)| component(registry, type_name, stored))
            .collect();
        let name = data
            .name
            .clone()
            .or_else(|| data.source_id.as_ref().map(|id| id.as_str().to_owned()))
            .unwrap_or_else(|| format!("{entity:?}"));
        let transform = data.transform_3d;
        let sprite = data
            .components
            .get("sindri.sprite")
            .and_then(|sprite| sprite.get("texture"))
            .and_then(Value::as_str)
            .map(|texture| sprite_region(texture, sheets));
        json!({
            "name": name,
            "id": data.source_id.as_ref().map(sindri_core::SceneEntityId::as_str),
            "position": transform.map(|transform| transform.position),
            "turn": transform.map(|transform| clockwise_degrees(transform.rotation)),
            "scale": transform.map(|transform| transform.scale),
            "tile": tile_size(world, registry),
            "sprite": sprite,
            "components": components,
            "undo": self.history.undo_label(),
            "redo": self.history.redo_label(),
        })
    }
}

fn component(registry: &ComponentSchemaRegistry, type_name: &str, stored: &Value) -> Value {
    let values = effective(registry, type_name, stored);
    let order = registry.field_order(type_name);
    let mut names: Vec<&String> = values
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, value)| value.is_number() || value.is_boolean())
        .map(|(name, _)| name)
        .collect();
    // The registry's order where it has one, so a component reads the same
    // way here as in the desktop inspector.
    names.sort_by_key(|name| {
        order
            .iter()
            .position(|known| *known == name.as_str())
            .unwrap_or(usize::MAX)
    });
    let fields: Vec<Value> = names
        .into_iter()
        .map(|name| json!({ "name": name, "value": values[name.as_str()] }))
        .collect();
    let meanings: Map<String, Value> = registry
        .meanings(type_name)
        .map(|(path, meaning)| (path.to_owned(), meaning_json(meaning)))
        .collect();
    let display = registry
        .metadata(type_name)
        .map_or(type_name, |metadata| metadata.display_name.as_str());
    json!({
        "type": type_name,
        "display": display,
        "values": values,
        "fields": fields,
        "meanings": meanings,
    })
}

/// The payload with every field the registry knows filled in at its default
/// beneath what is stored: what the engine actually uses.
pub fn effective(registry: &ComponentSchemaRegistry, type_name: &str, stored: &Value) -> Value {
    match registry.fields(type_name) {
        Some(defaults) => merged(defaults, stored),
        None => stored.clone(),
    }
}

/// `over` laid on `under`, object by object; anything else in `over` wins.
fn merged(under: &Value, over: &Value) -> Value {
    match (under, over) {
        (Value::Object(under), Value::Object(over)) => {
            let mut out = under.clone();
            for (key, value) in over {
                let laid = out
                    .get(key)
                    .map_or_else(|| value.clone(), |below| merged(below, value));
                out.insert(key.clone(), laid);
            }
            Value::Object(out)
        }
        _ => over.clone(),
    }
}

/// A sprite reference as the app thumbnails it: the texture, the named part
/// of it, and where that part is in the image (all of it with no part).
fn sprite_region(texture: &str, sheets: &BTreeMap<String, SpriteSheetDocument>) -> Value {
    let Ok(reference) = SpriteRef::parse(texture) else {
        return json!({ "texture": texture, "region": null, "rect": null });
    };
    let rect = match (reference.sprite(), reference.sheet()) {
        (None, _) => Some([0.0, 0.0, 1.0, 1.0]),
        (Some(region), Some(sheet)) => sheets
            .get(sheet.as_str())
            .and_then(|sheet| sheet.rects().ok())
            .and_then(|rects| rects.get(region).copied()),
        (Some(_), None) => None,
    };
    json!({ "texture": reference.texture(), "region": reference.sprite(), "rect": rect })
}

/// What a field means, for the app: its kind, and a range's ends (`null` for
/// an end the engine leaves open), a choice's spellings or a set's numbers.
fn meaning_json(meaning: &FieldMeaning) -> Value {
    let end = |value: f64| value.is_finite().then_some(value);
    match meaning {
        FieldMeaning::Range { min, max } => {
            json!({ "kind": "range", "min": end(*min), "max": end(*max) })
        }
        FieldMeaning::Choice(options) => json!({ "kind": "choice", "options": options }),
        FieldMeaning::OneOf(values) => json!({ "kind": "one_of", "values": values }),
        other => json!({ "kind": other.kind() }),
    }
}

/// A turn about the view axis, in degrees clockwise as the canvas shows it,
/// from `-180` to `180`.
fn clockwise_degrees(rotation: [f32; 4]) -> f32 {
    let (axis, angle) = glam::Quat::from_array(rotation).to_axis_angle();
    let counter = if axis.z < 0.0 { -angle } else { angle };
    let mut degrees = -counter.to_degrees();
    if degrees > 180.0 {
        degrees -= 360.0;
    } else if degrees <= -180.0 {
        degrees += 360.0;
    }
    degrees
}

/// World units across one tile of the scene's first tilemap, or one where it
/// has none: what "in tiles" means for this scene.
fn tile_size(world: &World, registry: &ComponentSchemaRegistry) -> f32 {
    registry
        .query::<TilemapComponent>(world)
        .ok()
        .and_then(|maps| maps.into_iter().next())
        .map_or(1.0, |(entity, map)| {
            let scale = world
                .world_transform(entity)
                .map_or(1.0, |transform| transform.scale[0].abs());
            let size = map.tile_size[0] * scale;
            if size.is_finite() && size > 0.0 {
                size
            } else {
                1.0
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_fields_lie_over_the_defaults_object_by_object() {
        let defaults =
            json!({ "radius": 1.0, "shape": { "shape": "box", "half_extents": [0.5, 0.5] } });
        let stored = json!({ "shape": { "half_extents": [2.0, 1.0] } });
        assert_eq!(
            merged(&defaults, &stored),
            json!({ "radius": 1.0, "shape": { "shape": "box", "half_extents": [2.0, 1.0] } })
        );
    }

    #[test]
    fn a_turn_reads_clockwise_in_degrees() {
        let quarter = glam::Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2).to_array();
        assert!((clockwise_degrees(quarter) - 90.0).abs() < 1e-3);
        assert!(clockwise_degrees(glam::Quat::IDENTITY.to_array()).abs() < 1e-3);
        let back = glam::Quat::from_rotation_z(0.5).to_array();
        assert!((clockwise_degrees(back) + 0.5_f32.to_degrees()).abs() < 1e-3);
    }
}
