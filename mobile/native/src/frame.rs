//! Fit the mobile canvas around the scene's world-space contents.

use glam::{Mat4, Quat, Vec3};
use sindri_core::{AssetId, EntityId, World, sheet_id_for};
use sindri_scene::{SpriteComponent, TilemapComponent};

use crate::{Editing, OpenedScene};

fn screen_ui(world: &World, entity: EntityId) -> bool {
    let mut current = Some(entity);
    while let Some(id) = current {
        let Some(data) = world.get(id) else { break; };
        if data.components.keys().any(|name| name.starts_with("sindri.ui.")) {
            return true;
        }
        current = data.parent;
    }
    false
}

fn include(bounds: &mut Option<[f32; 4]>, point: Vec3) {
    if !point.x.is_finite() || !point.y.is_finite() { return; }
    match bounds {
        Some([left, bottom, right, top]) => {
            *left = left.min(point.x);
            *bottom = bottom.min(point.y);
            *right = right.max(point.x);
            *top = top.max(point.y);
        }
        None => *bounds = Some([point.x, point.y, point.x, point.y]),
    }
}

fn sprite_bounds(opened: &OpenedScene, sprite: &SpriteComponent) -> [f32; 4] {
    if let Some(bounds) = opened.sprite_hits.bounds(sprite) { return bounds; }
    let offset = sprite.reference().ok()
        .and_then(|reference| reference.texture().parse::<AssetId>().ok())
        .and_then(|texture| sheet_id_for(&texture))
        .and_then(|sheet| opened.assets.sheets.get(sheet.as_str()).and_then(|sheet| sheet.anchor))
        .unwrap_or_default().offset();
    [offset[0] - 0.5, offset[1] - 0.5, offset[0] + 0.5, offset[1] + 0.5]
}

fn tile_bounds(map: &TilemapComponent) -> Option<[f32; 4]> {
    map.grid_bounds().ok()?;
    let last = [map.columns.saturating_sub(1), map.rows.saturating_sub(1)];
    let mut bounds = None;
    for [column, row] in [[0, 0], [last[0], 0], [0, last[1]], last] {
        let [x, y] = map.tile_to_local(column, row);
        include(&mut bounds, Vec3::new(x - map.tile_size[0] * 0.5, y - map.tile_size[1] * 0.5 - map.tile_overhang, 0.0));
        include(&mut bounds, Vec3::new(x + map.tile_size[0] * 0.5, y + map.tile_size[1] * 0.5, 0.0));
    }
    bounds
}

impl Editing {
    /// Fit visible world objects and editor markers to the canvas.
    pub fn frame_all(&mut self, opened: &OpenedScene, size: [f32; 2]) -> Result<(), String> {
        let mut bounds = None;
        let world = &opened.world;
        for (entity, _) in world.entities() {
            if screen_ui(world, entity) { continue; }
            let transform = world.world_transform(entity).unwrap_or_else(|| world.parent_space(entity));
            let matrix = Mat4::from_scale_rotation_translation(Vec3::from_array(transform.scale), Quat::from_array(transform.rotation), Vec3::from_array(transform.position));
            let sprite = opened.extractor.components().get::<SpriteComponent>(world, entity).map_err(|error| error.to_string())?;
            let map = opened.extractor.components().get::<TilemapComponent>(world, entity).map_err(|error| error.to_string())?;
            let rect = map.as_ref().and_then(tile_bounds).or_else(|| sprite.as_ref().map(|sprite| sprite_bounds(opened, sprite)));
            if let Some([left, bottom, right, top]) = rect {
                for [x, y] in [[left, bottom], [left, top], [right, bottom], [right, top]] {
                    include(&mut bounds, matrix.transform_point3(Vec3::new(x, y, 0.0)));
                }
            } else {
                include(&mut bounds, Vec3::from_array(transform.position));
            }
        }
        if let Some(bounds) = bounds { self.canvas.fit_bounds(bounds, size); }
        Ok(())
    }
}
