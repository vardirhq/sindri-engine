//! The edits the inspector makes: a component's field, found by its path, and
//! the selection's place (where it is, its turn, its size, its mirroring).
//!
//! Each is a checked command through the history, as on desktop. A slider
//! sends many values while a finger drags it; those merge into one undo step
//! (`merging`) until [`Editing::settle`] ends the run when the finger lifts.

use serde_json::Value;
use sindri_core::{CommandBuffer, Transform3D, World, WorldCommand};
use sindri_scene::SceneExtractor;

use crate::Editing;
use crate::inspector::effective;

/// A change to the selection's place, as the inspector's Place section makes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Place {
    /// Its position across the scene, in world units.
    Across(f32),
    /// Its position up the scene, in world units.
    Up(f32),
    /// Its turn about the view axis, in degrees clockwise as the canvas shows it.
    Turn(f32),
    /// Its size, as one scale for both axes; a mirrored axis stays mirrored.
    Size(f32),
    /// Whether it is drawn mirrored across.
    Mirror(bool),
}

impl Place {
    /// The change the app names `what` with the number or switch `value`.
    pub fn named(what: &str, value: &Value) -> Option<Self> {
        #[allow(clippy::cast_possible_truncation)] // Places, turns and sizes fit an f32.
        let number = || value.as_f64().map(|value| value as f32);
        match what {
            "across" => number().map(Self::Across),
            "up" => number().map(Self::Up),
            "turn" => number().map(Self::Turn),
            "size" => number().map(Self::Size),
            "mirror" => value.as_bool().map(Self::Mirror),
            _ => None,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Across(_) | Self::Up(_) => "Move",
            Self::Turn(_) => "Turn",
            Self::Size(_) => "Scale",
            Self::Mirror(_) => "Mirror",
        }
    }

    /// `transform` with this change made.
    fn applied(self, transform: Transform3D) -> Result<Transform3D, String> {
        let mut changed = transform;
        match self {
            Self::Across(x) => changed.position[0] = finite(x)?,
            Self::Up(y) => changed.position[1] = finite(y)?,
            Self::Turn(degrees) => {
                changed.rotation =
                    glam::Quat::from_rotation_z(-finite(degrees)?.to_radians()).to_array();
            }
            Self::Size(size) => {
                let size = finite(size)?;
                if size <= 0.0 {
                    return Err("a size must be above nothing".to_owned());
                }
                changed.scale[0] = size.copysign(transform.scale[0]);
                changed.scale[1] = size.copysign(transform.scale[1]);
            }
            Self::Mirror(mirrored) => {
                changed.scale[0] = if mirrored {
                    -transform.scale[0].abs()
                } else {
                    transform.scale[0].abs()
                };
            }
        }
        Ok(changed)
    }
}

fn finite(value: f32) -> Result<f32, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err("that is not a number".to_owned())
    }
}

impl Editing {
    /// Sets the field at `path` (`radius`, `shape.radius`, `pieces.0.sensor`)
    /// of the selection's component to `value`, refusing a value its schema
    /// would not decode. A field still at its default is written with it.
    ///
    /// # Errors
    /// Nothing selected, a component it does not carry, a path that is not
    /// there, a value the schema refuses, or a command the world refuses —
    /// each in words for the app.
    pub fn set_field(
        &mut self,
        world: &mut World,
        extractor: &SceneExtractor,
        type_name: &str,
        path: &str,
        value: Value,
        merging: bool,
    ) -> Result<(), String> {
        let entity = self.selected.ok_or("nothing is selected")?;
        let mut payload = world
            .get(entity)
            .and_then(|data| data.components.get(type_name))
            .cloned()
            .ok_or_else(|| format!("the selection has no {type_name}"))?;
        let registry = extractor.components();
        let values = effective(registry, type_name, &payload);
        set_path(&mut payload, &values, path, value)
            .ok_or_else(|| format!("{type_name} has no field {path}"))?;
        registry
            .validate_payload(type_name, &payload)
            .map_err(|error| format!("{path} cannot hold that value: {error}"))?;
        let label = registry
            .metadata(type_name)
            .map_or(type_name, |metadata| metadata.display_name.as_str());
        let mut buffer = CommandBuffer::new();
        buffer.push(WorldCommand::SetComponent {
            entity,
            type_name: type_name.to_owned(),
            payload,
        });
        let mut transaction = buffer.into_transaction(format!("Edit {label} {path}"));
        if merging {
            transaction = transaction.merging(format!("field:{entity:?}:{type_name}:{path}"));
        }
        self.history
            .apply(transaction, world)
            .map_err(|error| error.to_string())
    }

    /// Changes the selection's place: where it is, its turn, size or mirror.
    ///
    /// # Errors
    /// Nothing selected, a selection with no transform, a value that is not a
    /// number (or a size of nothing), or a command the world refuses.
    pub fn set_place(
        &mut self,
        world: &mut World,
        place: Place,
        merging: bool,
    ) -> Result<(), String> {
        let entity = self.selected.ok_or("nothing is selected")?;
        let transform = world
            .get(entity)
            .and_then(|data| data.transform_3d)
            .ok_or("the selection has no place to change")?;
        let mut buffer = CommandBuffer::new();
        buffer.push(WorldCommand::SetTransform3D {
            entity,
            transform: Some(place.applied(transform)?),
        });
        let mut transaction = buffer.into_transaction(place.label());
        if merging {
            transaction = transaction.merging(format!("place:{}:{entity:?}", place.label()));
        }
        self.history
            .apply(transaction, world)
            .map_err(|error| error.to_string())
    }

    /// A finger left a slider: the edits it merged become one finished step.
    pub fn settle(&mut self) {
        self.history.break_merge_run();
    }
}

/// Writes `value` at `path` in `payload`. A top-level field the payload does
/// not store yet is first copied from `values`, its effective value, so a
/// nested field of a default can be set. `None` where the path is not there.
fn set_path(payload: &mut Value, values: &Value, path: &str, value: Value) -> Option<()> {
    let mut steps = path.split('.');
    let first = steps.next()?;
    let object = payload.as_object_mut()?;
    let rest: Vec<&str> = steps.collect();
    if rest.is_empty() {
        object.insert(first.to_owned(), value);
        return Some(());
    }
    if !object.contains_key(first) {
        object.insert(first.to_owned(), values.get(first)?.clone());
    }
    let mut at = object.get_mut(first)?;
    for (index, step) in rest.iter().enumerate() {
        let last = index + 1 == rest.len();
        at = match at {
            Value::Object(fields) if last => {
                fields.insert((*step).to_owned(), value);
                return Some(());
            }
            Value::Object(fields) => fields.get_mut(*step)?,
            Value::Array(items) => {
                let slot = items.get_mut(step.parse::<usize>().ok()?)?;
                if last {
                    *slot = value;
                    return Some(());
                }
                slot
            }
            _ => return None,
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_path_reaches_into_objects_and_lists_and_fills_in_a_default() {
        let mut payload = json!({ "pieces": [{ "sensor": false }] });
        let values = json!({ "pieces": [{ "sensor": false }], "shape": { "shape": "box", "half_extents": [0.5, 0.5] } });
        set_path(&mut payload, &values, "pieces.0.sensor", json!(true)).unwrap();
        set_path(
            &mut payload,
            &values,
            "shape.half_extents",
            json!([1.0, 2.0]),
        )
        .unwrap();
        assert_eq!(
            payload,
            json!({ "pieces": [{ "sensor": true }], "shape": { "shape": "box", "half_extents": [1.0, 2.0] } })
        );
        assert!(set_path(&mut payload, &values, "pieces.3.sensor", json!(true)).is_none());
        assert!(set_path(&mut payload, &values, "nowhere.at_all", json!(1)).is_none());
    }

    #[test]
    fn size_keeps_a_mirror_and_mirror_keeps_the_size() {
        let start = Transform3D {
            scale: [-1.5, 1.5, 1.0],
            ..Transform3D::default()
        };
        let sized = Place::Size(2.0).applied(start).unwrap();
        assert_eq!(
            sized.scale.map(f32::to_bits),
            [-2.0_f32, 2.0, 1.0].map(f32::to_bits)
        );
        let unmirrored = Place::Mirror(false).applied(sized).unwrap();
        assert_eq!(
            unmirrored.scale.map(f32::to_bits),
            [2.0_f32, 2.0, 1.0].map(f32::to_bits)
        );
        assert!(Place::Size(0.0).applied(start).is_err());
    }
}
