//! Editor-only touch targets for objects without drawn world geometry.

use crate::Editing;
use crate::editing::Touch;
use serde_json::{Value, json};
use sindri_core::{EntityId, World};

/// World markers do not represent screen-space UI or already drawn geometry.
fn has_gizmo(world: &World, entity: EntityId) -> bool {
    let Some(data) = world.get(entity) else {
        return false;
    };
    if ["sindri.sprite", "sindri.mesh", "sindri.tilemap"]
        .iter()
        .any(|kind| data.components.contains_key(*kind))
    {
        return false;
    }
    let mut current = Some(entity);
    while let Some(id) = current {
        let Some(part) = world.get(id) else {
            break;
        };
        if part
            .components
            .keys()
            .any(|name| name.starts_with("sindri.ui."))
        {
            return false;
        }
        current = part.parent;
    }
    true
}

impl Editing {
    /// The visible non-sprite markers in screen pixels, for the editor overlay.
    pub fn gizmos(&self, world: &World, size: [f32; 2]) -> Vec<Value> {
        let per_pixel = self.canvas.per_pixel(size[1]);
        let selected = self.selected_entities();
        world.entities().filter(|(entity, _)| has_gizmo(world, *entity)).filter_map(|(entity, data)| {
            let position = world.world_transform(entity).unwrap_or_else(|| world.parent_space(entity)).position;
            let x = size[0] * 0.5 + (position[0] - self.canvas.center[0]) / per_pixel;
            let y = size[1] * 0.5 - (position[1] - self.canvas.center[1]) / per_pixel;
            if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 || x > size[0] || y > size[1] { return None; }
            let kind = if data.components.contains_key("sindri.light2d") { "light" } else if data.components.contains_key("sindri.camera") { "camera" } else { "object" };
            Some(json!({"id": entity.to_bits().to_string(), "name": data.name.as_deref().unwrap_or("Object"), "kind": kind, "x": x, "y": y, "selected": selected.contains(&entity)}))
        }).collect()
    }

    /// Select a marker and begin moving it; in multi-select mode toggle it instead.
    pub fn press_gizmo(&mut self, world: &World, id: &str, pixel: [f32; 2]) -> Result<(), String> {
        let bits = id.parse::<u64>().map_err(|_| "Invalid object handle")?;
        let entity = EntityId::from_bits(bits);
        if !has_gizmo(world, entity) {
            return Err("This object no longer has a marker".to_owned());
        }
        if self.selecting_more {
            self.toggle_selected(entity);
            self.invalidate_outline();
            self.touch = None;
            return Ok(());
        }
        self.clear_selection();
        self.selected = Some(entity);
        let start = world
            .world_transform(entity)
            .unwrap_or_else(|| world.parent_space(entity))
            .position;
        self.touch = Some(Touch::Moving {
            entity,
            from: pixel,
            start,
        });
        Ok(())
    }
}
