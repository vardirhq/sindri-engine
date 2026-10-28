//! `Grid.set_terrain` and `Grid.set_terrain_corner`: painting one of the map's
//! sheet's terrains into a cell, or one corner of a corners terrain.
//!
//! The cell is not the only thing that changes: every neighbor whose picture
//! depends on it is redrawn, which is `sindri_scene::paint_terrain`'s job and
//! the reason a script calls this rather than working out tiles itself. A
//! corner is the same for the tiles that meet there, and
//! `sindri_scene::paint_corner`'s.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::SceneComponent;
use sindri_scene::{TilemapComponent, paint_corner, paint_terrain};

use crate::surface::GridCall;

use super::WorldHost;
use super::convert::number;

impl WorldHost<'_> {
    pub(super) fn terrain_call(
        &mut self,
        call: GridCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let fail = |message: String| RuntimeError::Host(format!("{} {message}", path.dotted()));
        let map = self.entity_argument(path, args, 0, "the tilemap")?;
        let mut cell = [0_u32; 2];
        for (slot, which) in [(1, "a column"), (2, "a row")] {
            let value = number(
                path,
                args.get(slot)
                    .ok_or_else(|| fail(format!("needs {which}")))?,
            )?;
            if value < 0.0 || value.fract() != 0.0 || value > f64::from(u32::MAX) {
                return Err(fail(format!("needs a whole, non-negative {which}")));
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                cell[slot - 1] = value as u32;
            }
        }
        let corner = call == GridCall::SetTerrainCorner;
        let Some(Value::String(terrain)) = args.get(3) else {
            return Err(fail(if corner {
                "needs a corners terrain name".to_owned()
            } else {
                "needs a terrain name, or \"\" to empty the cell".to_owned()
            }));
        };
        let payload = self
            .world
            .get(map)
            .and_then(|data| data.components.get(TilemapComponent::TYPE_NAME))
            .ok_or_else(|| fail("was given an entity with no tilemap".to_owned()))?;
        let texture = payload
            .get("texture")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let terrains = self
            .tile_sets
            .and_then(|bindings| bindings.terrains().for_texture(&texture))
            .cloned()
            .unwrap_or_default();
        let payload = self
            .world
            .get_mut(map)
            .and_then(|data| data.components.get_mut(TilemapComponent::TYPE_NAME))
            .ok_or_else(|| fail("was given an entity with no tilemap".to_owned()))?;
        if corner {
            let Some(Value::Bool(set)) = args.get(4) else {
                return Err(fail(
                    "needs true to set the corner or false to clear it".to_owned(),
                ));
            };
            paint_corner(payload, &terrains, cell[0], cell[1], terrain, *set)
        } else {
            let wanted = (!terrain.is_empty()).then_some(terrain.as_str());
            paint_terrain(payload, &terrains, cell[0], cell[1], wanted)
        }
        .map_err(|error| fail(error.to_string()))?;
        Ok(Value::Unit)
    }
}
