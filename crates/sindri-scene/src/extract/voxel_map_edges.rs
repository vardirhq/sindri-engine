//! Soft edges between the kinds of ground on a map (`soft_edges`).
//!
//! A map draws each column as one square of its top block, so where meadow
//! meets heath the two meet on the grid's own straight seam, and a biome reads
//! as a pile of tiles rather than a place. With soft edges, every seam between
//! two different kinds of ground wavers: along it, short strips of one ground
//! are drawn a few pixels into the square beside it, so the line between them
//! bends one way and the other.
//!
//! A seam's line is a pair of sine bumps from its own seed plus a pixel of
//! jitter, so it is a wave rather than noise, and it is pinned to the seam at
//! both ends, which is where the seams of the squares around it meet: the
//! lines of neighbouring seams join. Strips are whole sixteenths of a square,
//! a pixel of a sixteen pixel block, so the edge stays pixel art.
//!
//! The ground creeping into a square is drawn with its own picture as it would
//! continue across the seam (a block's picture tiles), and with the light of
//! the square it lies on. Ground frays into water too, so a bank is as ragged
//! as the land, but water never creeps onto ground: what a square shows of
//! water is somewhere a walker could not stand.

use std::collections::HashMap;

use sindri_render::{TextureId, UvRect};
use sindri_voxel::VoxelId;

/// How many strips a seam is cut into, each an eighth of a square.
const STRIPS: i32 = 8;
/// One strip's length along the seam.
#[allow(clippy::cast_precision_loss)]
const STRIP: f32 = 1.0 / STRIPS as f32;
/// One pixel of a sixteen pixel block, the unit a strip reaches in by.
const PIXEL: f32 = 1.0 / 16.0;
/// The furthest a strip reaches into the square beside it, in pixels.
const FURTHEST: i32 = 6;

/// What a square needs known for its seams.
#[derive(Clone, Copy, Debug)]
pub(super) struct Ground {
    pub(super) voxel: VoxelId,
    pub(super) walkable: bool,
    pub(super) height: f32,
    pub(super) texture: TextureId,
    pub(super) rect: UvRect,
    pub(super) tint: [f32; 4],
}

/// A strip of one ground drawn into the square beside it.
#[derive(Clone, Copy, Debug)]
pub(super) struct MapPiece {
    /// The square it lies on.
    pub(super) column: i32,
    pub(super) row: i32,
    /// That square's height, so a flood that covers the square covers it.
    pub(super) height: f32,
    /// Its middle and size within the square, from the square's middle, with
    /// up the map north.
    pub(super) middle: [f32; 2],
    pub(super) size: [f32; 2],
    pub(super) texture: TextureId,
    pub(super) rect: UvRect,
    pub(super) tint: [f32; 4],
}

/// The strips that reach into the square at `column`, `row` from the
/// different ground around it. `ground` answers for that square and the four
/// beside it.
pub(super) fn into_square(
    seed: u64,
    column: i32,
    row: i32,
    ground: &HashMap<(i32, i32), Ground>,
) -> Vec<MapPiece> {
    let mut pieces = Vec::new();
    let Some(here) = ground.get(&(column, row)) else {
        return pieces;
    };
    // Each neighbour, whether the seam runs down the map (between columns),
    // and whether this square is the seam's first (west or north) square.
    for (dc, dr) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let Some(there) = ground.get(&(column + dc, row + dr)) else {
            continue;
        };
        if !there.walkable || there.voxel == here.voxel {
            continue;
        }
        // Into water, every strip of the seam reaches this way.
        let bank = !here.walkable;
        let down_the_map = dc != 0;
        let first = (column + dc.min(0), row + dr.min(0));
        let this_is_first = dc > 0 || dr > 0;
        for (strip, reach) in seam(seed, first, down_the_map).into_iter().enumerate() {
            // Positive reaches from the first square into the second.
            let into_this = if bank {
                reach.abs()
            } else if this_is_first {
                -reach
            } else {
                reach
            };
            if into_this <= 0 {
                continue;
            }
            #[allow(clippy::cast_precision_loss)]
            let depth = into_this as f32 * PIXEL;
            #[allow(clippy::cast_precision_loss)]
            let along = (strip as f32 + 0.5).mul_add(STRIP, -0.5);
            // The seam is on the side the neighbour is: west, east, north
            // (up the map) or south.
            let (middle, size) = if down_the_map {
                let x = (0.5 - depth / 2.0).copysign(f32::from(i8::try_from(dc).unwrap_or(0)));
                ([x, -along], [depth, STRIP])
            } else {
                let y = (0.5 - depth / 2.0).copysign(f32::from(i8::try_from(-dr).unwrap_or(0)));
                ([along, y], [STRIP, depth])
            };
            let Some(rect) = there.rect.part(middle, size) else {
                continue;
            };
            pieces.push(MapPiece {
                column,
                row,
                height: here.height,
                middle,
                size,
                texture: there.texture,
                rect,
                tint: here.tint,
            });
        }
    }
    pieces
}

/// How far each strip of a seam reaches, in pixels: positive from its first
/// square into its second, negative the other way.
///
/// `first` is the seam's west square (a seam down the map) or north one.
fn seam(seed: u64, first: (i32, i32), down_the_map: bool) -> [i32; STRIPS as usize] {
    let mut roll = mix(seed
        ^ (u64::from(first.0.cast_unsigned()) << 1)
        ^ (u64::from(first.1.cast_unsigned()) << 33)
        ^ u64::from(down_the_map));
    let mut next = |sides: u64| {
        roll = mix(roll);
        roll % sides
    };
    #[allow(clippy::cast_precision_loss)]
    let broad = next(9) as f32 - 4.0;
    #[allow(clippy::cast_precision_loss)]
    let ripple = next(5) as f32 - 2.0;
    let mut reaches = [0; STRIPS as usize];
    for (strip, reach) in reaches.iter_mut().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let t = (strip as f32 + 0.5) / STRIPS as f32;
        let wave = broad.mul_add(
            (std::f32::consts::PI * t).sin(),
            ripple * (std::f32::consts::TAU * t).sin(),
        );
        // A pixel of jitter inside, none at the ends, where seams meet.
        let jitter = if strip == 0 || strip == STRIPS as usize - 1 {
            0
        } else {
            i32::try_from(next(3)).unwrap_or(1) - 1
        };
        #[allow(clippy::cast_possible_truncation)]
        let pixels = wave.round() as i32 + jitter;
        *reach = pixels.clamp(-FURTHEST, FURTHEST);
    }
    reaches
}

/// `SplitMix64`'s finaliser: a well-spread number from any other.
const fn mix(value: u64) -> u64 {
    let mut z = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ground(voxel: u16, walkable: bool) -> Ground {
        Ground {
            voxel: VoxelId::new(voxel),
            walkable,
            height: 1.0,
            texture: TextureId::new(u32::from(voxel)),
            rect: UvRect::FULL,
            tint: [1.0; 4],
        }
    }

    /// A seam wavers but its ends stay on the grid, so neighbouring seams join.
    #[test]
    fn a_seam_wavers_and_is_pinned_at_its_ends() {
        let mut wavering = 0;
        for column in 0..40 {
            let reaches = seam(5, (column, 3), column % 2 == 0);
            assert!(
                reaches[0].abs() <= 2 && reaches[7].abs() <= 2,
                "{reaches:?}"
            );
            assert!(reaches.iter().all(|reach| reach.abs() <= FURTHEST));
            if reaches.iter().any(|reach| *reach != 0) {
                wavering += 1;
            }
        }
        assert!(wavering > 30, "most seams bend: {wavering} of 40");
        assert_eq!(
            seam(5, (2, 3), true),
            seam(5, (2, 3), true),
            "the same seed, the same line"
        );
    }

    /// Each strip of a seam is drawn once, on one side, by whichever square
    /// it reaches into, so the two squares of a seam agree.
    #[test]
    fn the_two_squares_of_a_seam_share_it_out() {
        let mut around = HashMap::new();
        around.insert((0, 0), ground(1, true));
        around.insert((1, 0), ground(2, true));
        let west = into_square(9, 0, 0, &around);
        let east = into_square(9, 1, 0, &around);
        let reaches = seam(9, (0, 0), true);
        let east_reaches = reaches.iter().filter(|reach| **reach > 0).count();
        let west_reaches = reaches.iter().filter(|reach| **reach < 0).count();
        assert_eq!(
            east.len(),
            east_reaches,
            "the west ground into the east square"
        );
        assert_eq!(
            west.len(),
            west_reaches,
            "and the east ground into the west one"
        );
        assert!(east.iter().all(|piece| piece.texture == TextureId::new(1)));
        assert!(west.iter().all(|piece| piece.texture == TextureId::new(2)));
        // Each lies against the seam: the west edge of the east square.
        for piece in &east {
            assert!((piece.middle[0] - piece.size[0] / 2.0 + 0.5).abs() < 1.0e-6);
        }
    }

    /// The same ground either side makes no edge to soften, and water never
    /// creeps onto ground, though ground frays into water.
    #[test]
    fn ground_frays_into_water_but_water_stays_off_ground() {
        let mut around = HashMap::new();
        around.insert((0, 0), ground(1, true));
        around.insert((1, 0), ground(1, true));
        around.insert((0, 1), ground(3, false));
        assert!(
            into_square(9, 0, 0, &around).is_empty(),
            "no water on the ground"
        );
        let bank = into_square(9, 0, 1, &around);
        let reaching = seam(9, (0, 0), false)
            .iter()
            .filter(|reach| **reach != 0)
            .count();
        assert_eq!(
            bank.len(),
            reaching,
            "every strip of the bank reaches into the water"
        );
        assert!(bank.iter().all(|piece| piece.texture == TextureId::new(1)));
    }
}
