//! Checking one face's visual: its sprite, its size and offset, and its
//! animation, each refused with the face it belongs to.

use crate::SpriteRef;

use super::{
    TILESET_FORMAT_VERSION, TileFace, TileFaceVisual, TileSetDocument, TileSetError, is_full_height,
};

fn validate_visual(
    tile: &str,
    face: TileFace,
    visual: &TileFaceVisual,
) -> Result<(), TileSetError> {
    if SpriteRef::parse(&visual.sprite).is_err() {
        return Err(TileSetError::InvalidSprite {
            tile: tile.to_owned(),
            face,
            sprite: visual.sprite.clone(),
        });
    }
    if !visual
        .size
        .iter()
        .all(|value| value.is_finite() && *value > 0.0)
    {
        return Err(TileSetError::InvalidSize {
            tile: tile.to_owned(),
            face,
        });
    }
    if let Some(animation) = &visual.animation {
        let broken = animation
            .frames
            .iter()
            .find(|frame| SpriteRef::parse(frame).is_err());
        if let Some(frame) = broken {
            return Err(TileSetError::InvalidSprite {
                tile: tile.to_owned(),
                face,
                sprite: frame.clone(),
            });
        }
        if animation.frames.is_empty() || !(animation.fps.is_finite() && animation.fps > 0.0) {
            return Err(TileSetError::InvalidAnimation {
                tile: tile.to_owned(),
                face,
            });
        }
    }
    if !visual.offset.iter().all(|value| value.is_finite()) {
        return Err(TileSetError::InvalidOffset {
            tile: tile.to_owned(),
            face,
        });
    }
    Ok(())
}

fn validate_model(tile: &str, definition: &super::TileDefinition) -> Result<(), TileSetError> {
    if definition.model.as_ref().is_some_and(|model| {
        let is_glb = std::path::Path::new(model.as_str())
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("glb"));
        !is_glb || !definition.fills_cell()
    }) {
        return Err(TileSetError::InvalidModel(tile.to_owned()));
    }
    Ok(())
}

impl TileSetDocument {
    pub fn validate(&self) -> Result<(), TileSetError> {
        if self.format_version != TILESET_FORMAT_VERSION {
            return Err(TileSetError::UnsupportedVersion {
                found: self.format_version,
                supported: TILESET_FORMAT_VERSION,
            });
        }
        if self.tiles.is_empty() {
            return Err(TileSetError::Empty);
        }
        for (tile, definition) in &self.tiles {
            if tile.trim().is_empty() {
                return Err(TileSetError::EmptyTileId);
            }

            if !definition.height.is_finite() || definition.height <= 0.0 || definition.height > 1.0
            {
                return Err(TileSetError::InvalidHeight {
                    tile: tile.clone(),
                    height: definition.height,
                });
            }
            // A height and a box are two answers to one question. Taking
            // either silently would make a document mean something its author
            // can only discover by looking at the picture.
            if definition.extent.is_some() && !is_full_height(&definition.height) {
                return Err(TileSetError::HeightAndExtent(tile.clone()));
            }
            if let Some(extent) = definition.extent {
                let sane = |axis: usize| {
                    let (low, high) = (extent.min[axis], extent.max[axis]);
                    low.is_finite()
                        && high.is_finite()
                        && (0.0..=1.0).contains(&low)
                        && (0.0..=1.0).contains(&high)
                        && high > low
                };
                if !(0..3).all(sane) {
                    return Err(TileSetError::InvalidExtent {
                        tile: tile.clone(),
                        extent,
                    });
                }
            }
            validate_model(tile, definition)?;
            if !(definition.glow.is_finite() && definition.glow >= 0.0) {
                return Err(TileSetError::InvalidGlow(tile.clone()));
            }
            if definition.walkable && !definition.supports {
                return Err(TileSetError::WalkableWithoutSupport(tile.clone()));
            }
            // Every look, not just the first: a variant with a broken sprite
            // is a hole that appears in whichever cells happen to choose it,
            // which is the worst kind of thing to find later.
            for faces in definition.all_faces() {
                if faces.iter().next().is_none() {
                    return Err(TileSetError::TileWithoutFaces(tile.clone()));
                }
                for (face, visual) in faces.iter() {
                    validate_visual(tile, face, visual)?;
                }
            }
        }
        Ok(())
    }
}
