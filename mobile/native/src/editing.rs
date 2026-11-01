//! Editing an opened scene by touch: what is selected, and the checked,
//! undoable commands a drag or an inspector field makes.
//!
//! Every change goes through `CommandHistory`, never a direct world write, as
//! on desktop (`editor/src/native/editing/mod.rs`): a drag is a run of moves
//! merged into one undo step, and a field edit is validated against the
//! component's schema before it is applied.

use serde_json::{Value, json};
use sindri_core::{CommandBuffer, CommandHistory, EntityId, Transform3D, World, WorldCommand};
use sindri_scene::SceneExtractor;
use sindri_scene::picking::{pick_world, world_bounds};

use crate::canvas::Canvas;
use crate::outline::outlined;

/// Which of the selection's handles a finger took.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    /// The round handle on a stem above: turns the selection to face the
    /// finger, with its top towards it.
    Turn,
    /// The round handle at the lower right: scales the selection by how far
    /// the finger is from its origin, against how far it started.
    Scale,
}

impl Handle {
    /// The handle named `turn` or `scale`, as the app sends it.
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "turn" => Some(Self::Turn),
            "scale" => Some(Self::Scale),
            _ => None,
        }
    }
}

/// A scale is kept between these, so a handle cannot shrink something out of
/// reach or blow it up past any use.
const SMALLEST_SCALE: f32 = 0.05;
const LARGEST_SCALE: f32 = 100.0;

/// What the outline was last built for, so it is built again only when one of
/// them changes rather than on every frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OutlineKey {
    revision: u64,
    entity: EntityId,
    entity_revision: Option<u64>,
    width: u32,
}

/// What one touch on the canvas is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Touch {
    /// Moving the view, because it did not start on the selection.
    Panning { last: [f32; 2] },
    /// Moving the selected entity, by how far the finger has gone from where
    /// it landed, from where the entity started.
    Moving {
        entity: EntityId,
        from: [f32; 2],
        start: [f32; 3],
    },
    /// Turning or scaling the selected entity by one of its handles, about
    /// `pivot` on screen, from the transform it had when the handle was taken.
    Handling {
        handle: Handle,
        entity: EntityId,
        pivot: [f32; 2],
        from: [f32; 2],
        start: Transform3D,
        snap: bool,
    },
}

/// An opened scene being edited: its history, its selection, and the view.
pub struct Editing {
    pub history: CommandHistory,
    pub selected: Option<EntityId>,
    pub canvas: Canvas,
    /// Whether the canvas shows the scene's 2D lights and the darkness they
    /// light. Off by default, as the design's menu has it, so a scene that is
    /// dark on purpose can still be seen to be worked on.
    pub lighting_preview: bool,
    touch: Option<Touch>,
    /// The world the canvas last drew with the selection outlined, and what it
    /// was built for.
    outline: Option<(OutlineKey, Option<World>)>,
    /// The history revision the file was last agreed with: opened at, or saved
    /// at. Compared rather than flagged, as desktop does, so undoing back to
    /// the saved state reads as saved again.
    saved_revision: u64,
}

impl Editing {
    /// A session over `world`, viewed as its own camera frames it.
    ///
    /// # Errors
    /// As [`Canvas::framing`].
    pub fn new(world: &World, extractor: &SceneExtractor) -> Result<Self, String> {
        Ok(Self {
            history: CommandHistory::default(),
            selected: None,
            canvas: Canvas::framing(world, extractor)?,
            lighting_preview: false,
            touch: None,
            outline: None,
            saved_revision: 0,
        })
    }

    /// Whether the world holds edits its file does not.
    pub fn unsaved(&self) -> bool {
        self.history.revision() != self.saved_revision
    }

    /// The world was just written to its file.
    pub fn saved(&mut self) {
        self.saved_revision = self.history.revision();
    }

    /// The entity drawn topmost under `pixel`, through the frame's own camera.
    ///
    /// # Errors
    /// A camera that does not resolve, or a component that does not decode.
    pub fn pick(
        &self,
        world: &World,
        extractor: &SceneExtractor,
        pixel: [f32; 2],
        size: [f32; 2],
    ) -> Result<Option<EntityId>, String> {
        let camera = self.canvas.camera(world, extractor, size)?;
        let point = [pixel[0] / size[0].max(1.0), pixel[1] / size[1].max(1.0)];
        pick_world(world, extractor.components(), camera.view_projection, point)
            .map_err(|error| error.to_string())
    }

    /// A tap: selects what is under it, or clears the selection over nothing.
    ///
    /// # Errors
    /// As [`Self::pick`].
    pub fn tap(
        &mut self,
        world: &World,
        extractor: &SceneExtractor,
        pixel: [f32; 2],
        size: [f32; 2],
    ) -> Result<(), String> {
        self.selected = self.pick(world, extractor, pixel, size)?;
        Ok(())
    }

    /// A finger landed. It moves the selection if it landed on it, and the view
    /// otherwise, so nothing moves that was not chosen first.
    ///
    /// # Errors
    /// As [`Self::pick`].
    pub fn press(
        &mut self,
        world: &World,
        extractor: &SceneExtractor,
        pixel: [f32; 2],
        size: [f32; 2],
    ) -> Result<(), String> {
        let under = self.pick(world, extractor, pixel, size)?;
        let start = self
            .selected
            .filter(|selected| under == Some(*selected))
            .and_then(|entity| {
                let transform = world.get(entity)?.transform_3d?;
                Some((entity, transform.position))
            });
        self.touch = Some(match start {
            Some((entity, start)) => Touch::Moving {
                entity,
                from: pixel,
                start,
            },
            None => Touch::Panning { last: pixel },
        });
        Ok(())
    }

    /// The finger moved to `pixel`.
    ///
    /// # Errors
    /// The move was refused by the world.
    pub fn drag(
        &mut self,
        world: &mut World,
        pixel: [f32; 2],
        size: [f32; 2],
    ) -> Result<(), String> {
        match self.touch {
            Some(Touch::Panning { last }) => {
                self.canvas
                    .pan([pixel[0] - last[0], pixel[1] - last[1]], size);
                self.touch = Some(Touch::Panning { last: pixel });
                Ok(())
            }
            Some(Touch::Moving {
                entity,
                from,
                start,
            }) => {
                let per_pixel = self.canvas.per_pixel(size[1]);
                let Some(mut transform) = world.get(entity).and_then(|data| data.transform_3d)
                else {
                    return Ok(());
                };
                transform.position = [
                    start[0] + (pixel[0] - from[0]) * per_pixel,
                    start[1] - (pixel[1] - from[1]) * per_pixel,
                    start[2],
                ];
                let mut buffer = CommandBuffer::new();
                buffer.push(WorldCommand::SetTransform3D {
                    entity,
                    transform: Some(transform),
                });
                let transaction = buffer
                    .into_transaction("Move")
                    .merging(format!("touch-move:{entity:?}"));
                self.history
                    .apply(transaction, world)
                    .map_err(|error| error.to_string())
            }
            Some(Touch::Handling {
                handle,
                entity,
                pivot,
                from,
                start,
                snap,
            }) => {
                let transform = match handle {
                    Handle::Turn => turned(start, pivot, pixel, snap),
                    Handle::Scale => scaled(start, pivot, from, pixel, snap),
                };
                let mut buffer = CommandBuffer::new();
                buffer.push(WorldCommand::SetTransform3D {
                    entity,
                    transform: Some(transform),
                });
                let (label, merge) = match handle {
                    Handle::Turn => ("Turn", "touch-turn"),
                    Handle::Scale => ("Scale", "touch-scale"),
                };
                let transaction = buffer
                    .into_transaction(label)
                    .merging(format!("{merge}:{entity:?}"));
                self.history
                    .apply(transaction, world)
                    .map_err(|error| error.to_string())
            }
            None => Ok(()),
        }
    }

    /// A finger took one of the selection's handles at `pixel`. Dragging then
    /// turns or scales the selection rather than moving it or the view, and
    /// `snap` keeps a turn to fifteen degrees and a scale to tenths.
    ///
    /// # Errors
    /// Nothing selected, or a selection with no transform to change.
    pub fn take_handle(
        &mut self,
        world: &World,
        extractor: &SceneExtractor,
        handle: Handle,
        pixel: [f32; 2],
        size: [f32; 2],
        snap: bool,
    ) -> Result<(), String> {
        let entity = self.selected.ok_or("nothing is selected")?;
        let start = world
            .get(entity)
            .and_then(|data| data.transform_3d)
            .ok_or("the selection has no transform to change")?;
        let origin = self
            .selection_pixel(world, size)
            .ok_or("the selection has no transform to change")?;
        // A turn is about the middle of what is drawn, which is where the
        // eye puts it; a scale is about the origin, which is what stays put.
        let pivot = match (handle, self.selection_bounds(world, extractor, size)?) {
            (Handle::Turn, Some([left, top, right, bottom])) => {
                [(left + right) * 0.5, (top + bottom) * 0.5]
            }
            _ => origin,
        };
        self.touch = Some(Touch::Handling {
            handle,
            entity,
            pivot,
            from: pixel,
            start,
            snap,
        });
        Ok(())
    }

    /// The finger lifted: a move, turn or scale in progress becomes one
    /// finished undo step.
    pub fn release(&mut self) {
        if matches!(
            self.touch.take(),
            Some(Touch::Moving { .. } | Touch::Handling { .. })
        ) {
            self.history.break_merge_run();
        }
    }

    /// The world the canvas draws: `world` itself, or a copy with the
    /// selection outlined `width` pixels wide around its shape. The copy is
    /// kept until the selection, an edit or the zoom changes it.
    pub fn drawn<'a>(&'a mut self, world: &'a World, size: [f32; 2], width: f32) -> &'a World {
        let Some(entity) = self.selected else {
            self.outline = None;
            return world;
        };
        let width = width * self.canvas.per_pixel(size[1]);
        let key = OutlineKey {
            revision: self.history.revision(),
            entity,
            entity_revision: world.revision(entity),
            width: width.to_bits(),
        };
        if self.outline.as_ref().map(|(built, _)| *built) != Some(key) {
            self.outline = Some((key, outlined(world, entity, width)));
        }
        self.outline
            .as_ref()
            .and_then(|(_, outlined)| outlined.as_ref())
            .unwrap_or(world)
    }

    /// Sets one top-level field of the selected entity's component, refusing a
    /// value the component's schema would not decode.
    ///
    /// # Errors
    /// Nothing selected, a component it does not carry, a value the schema
    /// refuses, or a command the world refuses — each in words for the app.
    pub fn set_field(
        &mut self,
        world: &mut World,
        extractor: &SceneExtractor,
        type_name: &str,
        field: &str,
        value: Value,
    ) -> Result<(), String> {
        let entity = self.selected.ok_or("nothing is selected")?;
        let mut payload = world
            .get(entity)
            .and_then(|data| data.components.get(type_name))
            .cloned()
            .ok_or_else(|| format!("the selection has no {type_name}"))?;
        let object = payload
            .as_object_mut()
            .ok_or_else(|| format!("{type_name} has no fields to set"))?;
        object.insert(field.to_owned(), value);
        extractor
            .components()
            .validate_payload(type_name, &payload)
            .map_err(|error| format!("{field} cannot hold that value: {error}"))?;
        let label = extractor
            .components()
            .metadata(type_name)
            .map_or(type_name, |metadata| metadata.display_name.as_str());
        let mut buffer = CommandBuffer::new();
        buffer.push(WorldCommand::SetComponent {
            entity,
            type_name: type_name.to_owned(),
            payload,
        });
        self.history
            .apply(
                buffer.into_transaction(format!("Edit {label} {field}")),
                world,
            )
            .map_err(|error| error.to_string())
    }

    /// Undoes the last step, if there is one.
    ///
    /// # Errors
    /// The world refused the reversal.
    pub fn undo(&mut self, world: &mut World) -> Result<(), String> {
        self.touch = None;
        self.history
            .undo(world)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Redoes the last undone step, if there is one.
    ///
    /// # Errors
    /// The world refused the step.
    pub fn redo(&mut self, world: &mut World) -> Result<(), String> {
        self.touch = None;
        self.history
            .redo(world)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// What the app's inspector sheet shows, as JSON: the selection's name and
    /// position, and each component's top-level numbers and switches, which are
    /// the fields the first slice edits. `null` when nothing is selected.
    pub fn inspector(&self, world: &World, extractor: &SceneExtractor) -> Value {
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
            .filter_map(|(type_name, payload)| {
                let object = payload.as_object()?;
                let order = registry.field_order(type_name);
                let mut names: Vec<&String> = object
                    .iter()
                    .filter(|(_, value)| value.is_number() || value.is_boolean())
                    .map(|(name, _)| name)
                    .collect();
                // The registry's order where it has one, so a component reads
                // the same way here as in the desktop inspector.
                names.sort_by_key(|name| {
                    order
                        .iter()
                        .position(|known| *known == name.as_str())
                        .unwrap_or(usize::MAX)
                });
                let fields: Vec<Value> = names
                    .into_iter()
                    .map(|name| json!({ "name": name, "value": object[name] }))
                    .collect();
                let display = registry
                    .metadata(type_name)
                    .map_or(type_name.as_str(), |metadata| {
                        metadata.display_name.as_str()
                    });
                Some(json!({ "type": type_name, "display": display, "fields": fields }))
            })
            .collect();
        let name = data
            .name
            .clone()
            .or_else(|| data.source_id.as_ref().map(|id| id.as_str().to_owned()))
            .unwrap_or_else(|| format!("{entity:?}"));
        json!({
            "name": name,
            "position": data.transform_3d.map(|transform| transform.position),
            "components": components,
            "undo": self.history.undo_label(),
            "redo": self.history.redo_label(),
        })
    }

    /// Where the selection's origin is on screen, in pixels, for the app to
    /// draw its marker. `None` with nothing selected or off the camera.
    pub fn selection_pixel(&self, world: &World, size: [f32; 2]) -> Option<[f32; 2]> {
        let position = world.get(self.selected?)?.transform_3d?.position;
        let per_pixel = self.canvas.per_pixel(size[1]);
        Some([
            size[0] * 0.5 + (position[0] - self.canvas.center[0]) / per_pixel,
            size[1] * 0.5 - (position[1] - self.canvas.center[1]) / per_pixel,
        ])
    }

    /// The box the selection covers on screen, in pixels: left, top, right
    /// and bottom, for the app to outline. The same shape a tap on it hits.
    /// `None` with nothing selected, nothing drawn, or nothing on screen.
    ///
    /// # Errors
    /// A camera that does not resolve, or a component that does not decode.
    pub fn selection_bounds(
        &self,
        world: &World,
        extractor: &SceneExtractor,
        size: [f32; 2],
    ) -> Result<Option<[f32; 4]>, String> {
        let Some(entity) = self.selected else {
            return Ok(None);
        };
        let camera = self.canvas.camera(world, extractor, size)?;
        let bounds = world_bounds(
            world,
            extractor.components(),
            camera.view_projection,
            entity,
        )
        .map_err(|error| error.to_string())?;
        Ok(bounds.map(|[left, top, right, bottom]| {
            [
                left * size[0],
                top * size[1],
                right * size[0],
                bottom * size[1],
            ]
        }))
    }
}

/// `start` turned so its top faces `finger`, measured about `pivot` on screen:
/// straight up is no turn, and a finger to the right turns it clockwise.
fn turned(start: Transform3D, pivot: [f32; 2], finger: [f32; 2], snap: bool) -> Transform3D {
    let (across, up) = (finger[0] - pivot[0], pivot[1] - finger[1]);
    if across.hypot(up) < 1.0 {
        return start;
    }
    let mut clockwise = across.atan2(up);
    if snap {
        let step = 15_f32.to_radians();
        clockwise = (clockwise / step).round() * step;
    }
    Transform3D {
        rotation: glam::Quat::from_rotation_z(-clockwise).to_array(),
        ..start
    }
}

/// `start` scaled across and up by how far `finger` is from `pivot`, against
/// how far `from` was, keeping a mirrored sprite mirrored.
fn scaled(
    start: Transform3D,
    pivot: [f32; 2],
    from: [f32; 2],
    finger: [f32; 2],
    snap: bool,
) -> Transform3D {
    let began = (from[0] - pivot[0]).hypot(from[1] - pivot[1]);
    if began < 1.0 {
        return start;
    }
    let factor = (finger[0] - pivot[0]).hypot(finger[1] - pivot[1]) / began;
    let axis = |value: f32| {
        let mut size = (value.abs() * factor).clamp(SMALLEST_SCALE, LARGEST_SCALE);
        if snap {
            size = ((size * 10.0).round() / 10.0).max(SMALLEST_SCALE);
        }
        size.copysign(value)
    };
    let [x, y, z] = start.scale;
    Transform3D {
        scale: [axis(x), axis(y), z],
        ..start
    }
}
