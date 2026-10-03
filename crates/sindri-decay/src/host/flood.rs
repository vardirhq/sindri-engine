//! A reversible water overlay on a voxel map, independent of its generator.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use serde_json::json;
use sindri_core::SceneComponent;
use sindri_scene::{VoxelBlock, VoxelView, VoxelWorldComponent};

use super::{WorldHost, convert::number};
use crate::surface::GridCall;

impl WorldHost<'_> {
    pub(super) fn flood_call(
        &mut self,
        call: GridCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let entity = self.entity_argument(path, args, 0, "the voxel map")?;
        let payload = self
            .world
            .get(entity)
            .and_then(|data| data.components.get(VoxelWorldComponent::TYPE_NAME))
            .ok_or_else(|| RuntimeError::Host(format!("{} needs a voxel map", path.dotted())))?;
        let component: VoxelWorldComponent = serde_json::from_value(payload.clone())
            .map_err(|error| RuntimeError::Host(error.to_string()))?;
        if component.view != VoxelView::Map {
            return Err(RuntimeError::Host(format!(
                "{} needs view: map",
                path.dotted()
            )));
        }
        let ground = self
            .voxel_ground(path, entity)?
            .expect("component read above");
        if call == GridCall::Flooded {
            let [column, row, _] = Self::cell_argument(path, args, 2)?;
            return Ok(Value::Bool(component.map_flood.is_some_and(|flood| {
                ground
                    .surface(column, row)
                    .is_some_and(|(_, height, _)| height < flood.level)
            })));
        }
        let level = number(
            path,
            args.get(1)
                .ok_or_else(|| RuntimeError::Host("missing flood level".into()))?,
        )?;
        if !level.is_finite() || level.abs() > f64::from(f32::MAX) {
            return Err(RuntimeError::Host(
                "flood level must be a finite f32".into(),
            ));
        }
        let Some(Value::String(block)) = args.get(2) else {
            return Err(RuntimeError::Host(
                "flood needs a water block name or an empty string".into(),
            ));
        };
        if !block.is_empty() {
            ground
                .id_of(&VoxelBlock::Named(block.clone()))
                .map_err(|error| RuntimeError::Host(error.to_string()))?;
        }
        let flood = if block.is_empty() {
            serde_json::Value::Null
        } else {
            json!({"level": level, "block": block})
        };
        // Avoid marking the entity changed for an identical level (low/high tide).
        let old = self
            .world
            .get(entity)
            .and_then(|data| data.components.get(VoxelWorldComponent::TYPE_NAME))
            .and_then(|payload| payload.get("map_flood"));
        if old != Some(&flood) {
            let payload = self
                .world
                .get_mut(entity)
                .and_then(|data| data.components.get_mut(VoxelWorldComponent::TYPE_NAME))
                .expect("component read above");
            payload["map_flood"] = flood;
        }
        Ok(Value::Unit)
    }
}
