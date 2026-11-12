//! What a finger lands on: the drawn art, and the editor's markers for objects
//! with nothing drawn.
//!
//! A marker is shown only for a thing that has no other way to be tapped: an
//! object with a place in the world and nothing drawn anywhere in its group.
//! The parts of a drawn object, its lights and particles, are reached through
//! the object or the Layers sheet, so they do not crowd it with markers. The
//! selection always shows one, so a part picked in Layers can still be
//! dragged.
//!
//! The engine decides what a touch hits, drawn art first, and markers by
//! distance after it. Tapping the same spot again moves the selection to the
//! next thing under the finger, so a light sitting on a lamp is one tap
//! further. A marker obeys the canvas's rule: tap to select, then drag to move;
//! a drag that starts on something not selected pans.

use std::collections::BTreeSet;

use crate::Editing;
use crate::editing::Touch;
use serde_json::{Value, json};
use sindri_core::{EntityId, World};
use sindri_scene::SceneExtractor;

/// A marker's reach before the host has said how dense the screen is:
/// eighteen points at three pixels a point.
pub const DEFAULT_MARKER_RADIUS: f32 = 54.0;

/// Components that are drawn and can be tapped on the canvas themselves.
const DRAWN: [&str; 3] = ["sindri.sprite", "sindri.mesh", "sindri.tilemap"];

fn drawn(world: &World, entity: EntityId) -> bool {
    world
        .get(entity)
        .is_some_and(|data| DRAWN.iter().any(|kind| data.components.contains_key(*kind)))
}

fn in_screen_ui(world: &World, entity: EntityId) -> bool {
    let mut current = Some(entity);
    while let Some(id) = current {
        let Some(part) = world.get(id) else {
            return false;
        };
        if part
            .components
            .keys()
            .any(|name| name.starts_with("sindri.ui."))
        {
            return true;
        }
        current = part.parent;
    }
    false
}

fn root_of(world: &World, entity: EntityId) -> EntityId {
    let mut root = entity;
    while let Some(parent) = world.get(root).and_then(|data| data.parent) {
        root = parent;
    }
    root
}

/// The top-level objects with something drawn anywhere in their family: the
/// art is what a finger finds them by, and their other parts are reached
/// through it or the Layers sheet.
fn drawn_families(world: &World) -> BTreeSet<EntityId> {
    world
        .entities()
        .map(|(entity, _)| entity)
        .filter(|entity| drawn(world, *entity))
        .map(|entity| root_of(world, entity))
        .collect()
}

/// Whether `entity` has a place in the world and nothing drawn of its own.
fn markable(world: &World, entity: EntityId) -> bool {
    world
        .get(entity)
        .is_some_and(|data| data.transform_3d.is_some())
        && !drawn(world, entity)
        && !in_screen_ui(world, entity)
}

impl Editing {
    /// The entities that show a marker: markable ones in families with
    /// nothing drawn, and the selection when it is markable.
    pub fn marked(&self, world: &World) -> Vec<EntityId> {
        let families = drawn_families(world);
        let selected = self.selected_entities();
        world
            .entities()
            .map(|(entity, _)| entity)
            .filter(|entity| markable(world, *entity))
            .filter(|entity| {
                !families.contains(&root_of(world, *entity)) || selected.contains(entity)
            })
            .collect()
    }

    fn marker_pixel(&self, world: &World, entity: EntityId, size: [f32; 2]) -> Option<[f32; 2]> {
        let per_pixel = self.canvas.per_pixel(size[1]);
        let position = world
            .world_transform(entity)
            .unwrap_or_else(|| world.parent_space(entity))
            .position;
        let x = size[0] * 0.5 + (position[0] - self.canvas.center[0]) / per_pixel;
        let y = size[1] * 0.5 - (position[1] - self.canvas.center[1]) / per_pixel;
        (x.is_finite()
            && y.is_finite()
            && (0.0..=size[0]).contains(&x)
            && (0.0..=size[1]).contains(&y))
        .then_some([x, y])
    }

    /// The visible markers in screen pixels, for the editor overlay to draw.
    pub fn gizmos(&self, world: &World, size: [f32; 2]) -> Vec<Value> {
        let selected = self.selected_entities();
        self.marked(world)
            .into_iter()
            .filter_map(|entity| {
                let [x, y] = self.marker_pixel(world, entity, size)?;
                let data = world.get(entity)?;
                let kind = if data.components.contains_key("sindri.light2d") {
                    "light"
                } else if data.components.contains_key("sindri.camera") {
                    "camera"
                } else {
                    "object"
                };
                Some(json!({
                    "id": entity.to_bits().to_string(),
                    "name": data.name.as_deref().unwrap_or("Object"),
                    "kind": kind,
                    "x": x,
                    "y": y,
                    "selected": selected.contains(&entity),
                }))
            })
            .collect()
    }

    /// Everything under `pixel`, in the order a tap offers it: the drawn art
    /// on top, then markers nearest first.
    ///
    /// # Errors
    /// As [`Self::pick`].
    pub fn under(
        &self,
        world: &World,
        extractor: &SceneExtractor,
        pixel: [f32; 2],
        size: [f32; 2],
    ) -> Result<Vec<EntityId>, String> {
        let mut under: Vec<EntityId> = self
            .pick(world, extractor, pixel, size)?
            .into_iter()
            .collect();
        let mut markers: Vec<(f32, EntityId)> = self
            .marked(world)
            .into_iter()
            .filter_map(|entity| {
                let at = self.marker_pixel(world, entity, size)?;
                let distance = (at[0] - pixel[0]).hypot(at[1] - pixel[1]);
                (distance <= self.marker_radius).then_some((distance, entity))
            })
            .collect();
        markers.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, entity) in markers {
            if !under.contains(&entity) {
                under.push(entity);
            }
        }
        Ok(under)
    }

    /// A tap: selects what is under it, the next thing under it when the
    /// selection already is, or clears the selection over nothing.
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
        let under = self.under(world, extractor, pixel, size)?;
        if self.selecting_more {
            if let Some(entity) = under.first() {
                self.toggle_selected(*entity);
            }
        } else {
            let next = self
                .selected
                .filter(|_| self.additional.is_empty())
                .and_then(|selected| under.iter().position(|entity| *entity == selected))
                .map_or(0, |index| (index + 1) % under.len());
            self.selected = under.get(next).copied();
            self.additional.clear();
        }
        self.invalidate_outline();
        Ok(())
    }

    /// A finger landed. It moves the selection if it landed on it, its art or
    /// its marker, and the view otherwise, so nothing moves that was not
    /// chosen first.
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
        let under = self.under(world, extractor, pixel, size)?;
        let start = self
            .selected
            .filter(|_| self.additional.is_empty() && !self.selecting_more)
            .filter(|selected| under.contains(selected))
            .filter(|selected| world.get(*selected).is_some())
            .map(|entity| {
                let position = world
                    .world_transform(entity)
                    .unwrap_or_else(|| world.parent_space(entity))
                    .position;
                (entity, position)
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
}
