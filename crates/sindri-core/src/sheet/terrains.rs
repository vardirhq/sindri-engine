//! A sheet's terrains, checked against the sprites it cuts.

use std::collections::BTreeMap;

use super::{SheetError, SpriteSheetDocument};
use crate::TerrainKind;

impl SpriteSheetDocument {
    /// Every terrain lists as many sprites as its kind answers, and each is
    /// one the sheet cuts.
    pub(super) fn check_terrains(
        &self,
        rects: &BTreeMap<String, [f32; 4]>,
    ) -> Result<(), SheetError> {
        for (name, terrain) in &self.terrains {
            let wanted = terrain.kind.sprites();
            if terrain.sprites.len() != wanted {
                return Err(SheetError::TerrainSize {
                    terrain: name.clone(),
                    kind: match terrain.kind {
                        TerrainKind::Edges => "edges",
                        TerrainKind::Blob => "blob",
                        TerrainKind::Corners => "corners",
                    },
                    found: terrain.sprites.len(),
                    wanted,
                });
            }
            if let Some(missing) = terrain
                .sprites
                .iter()
                .find(|sprite| !rects.contains_key(*sprite))
            {
                return Err(SheetError::TerrainSprite {
                    terrain: name.clone(),
                    sprite: missing.clone(),
                });
            }
        }
        Ok(())
    }
}
