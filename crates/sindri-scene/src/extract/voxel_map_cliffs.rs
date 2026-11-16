//! Drops a map's ground makes, shown as faces (`map_cliffs`).
//!
//! A map is drawn flat, and a terrace or a cliff is otherwise only a darker
//! line. Where a square stands above the one south of it (nearer a camera
//! that looks north), the face of that drop is drawn as a band across the
//! top of the lower square: the higher block's side, in shade, as tall as the
//! drop, up to most of a square. Seen from behind and above, as the wilds'
//! camera sees it, a heath reads as raised ground.

use std::collections::HashMap;

use sindri_voxel::{VoxelCoord, VoxelFace};

use crate::VoxelTextureSource;

use super::{MapPiece, MapSquare, MapWorld, square};

/// The least drop that is a cliff rather than a slope: a gentle hillside
/// steps down a level at a time, and drawing each step would stripe it.
const LEAST: f32 = 2.0;
/// How much of a square each level of a cliff past the first covers, and the
/// most a face may.
const PER_LEVEL: f32 = 0.3;
const TALLEST: f32 = 0.8;
/// How much darker a face is than the ground above it: it faces away from
/// a sun in the north west.
const SHADE: f32 = 0.6;

/// The faces of the drops from a chunk's squares to the squares south of
/// them, which reach one row past the chunk.
pub(super) fn faces(map: &MapWorld, squares: &[MapSquare], last_row: i32) -> Vec<MapPiece> {
    let by_place: HashMap<(i32, i32), &MapSquare> = squares
        .iter()
        .map(|square| ((square.column, square.row), square))
        .collect();
    let mut out = Vec::new();
    for high in squares {
        let below = (high.column, high.row + 1);
        let south = match by_place.get(&below) {
            Some(found) => Some(**found),
            None if high.row == last_row => square(map, below.0, below.1),
            None => None,
        };
        let Some(low) = south else {
            continue;
        };
        let drop = high.height - low.height;
        if drop < LEAST {
            continue;
        }
        let tall = ((drop - 1.0) * PER_LEVEL).min(TALLEST);
        let Some((level, _, _)) = map.ground.surface(high.column, high.row) else {
            continue;
        };
        let face = map.resolved.texture_at(
            high.voxel,
            VoxelFace::Front,
            VoxelCoord::new(high.column, level, high.row),
            false,
        );
        let [r, g, b, a] = high.tint;
        out.push(MapPiece {
            column: low.column,
            row: low.row,
            height: low.height,
            middle: [0.0, 0.5 - tall / 2.0],
            size: [1.0, tall],
            texture: face.texture,
            rect: face.uv,
            tint: [r * SHADE, g * SHADE, b * SHADE, a],
        });
    }
    out
}
