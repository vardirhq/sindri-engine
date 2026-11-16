//! Places a scatter stands whole: a hamlet, a ruin, a camp (`sindri.scatter`'s
//! `sites`).
//!
//! A rule stands one thing to a patch; a site stands a layout. The land is cut
//! into patches `spacing` squares on a side, and the seed chooses whether each
//! holds one and where in it its middle is. A middle is only taken where the
//! site's ground is under it and the land round it, out to its `radius`, can be
//! walked on, so a hamlet is not built across a river; the seed tries a few
//! places in a patch before it gives that patch up. The seed also chooses
//! whether the layout is mirrored and which of its pieces stand, so no two
//! sites of one kind are quite the same. Nothing a rule grows stands inside a
//! site.

use serde::{Deserialize, Serialize};

use super::scatter::{Placement, Roll, ScatterComponent};
use crate::VoxelGround;

/// One kind of place, and where it is built.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ScatterSite {
    /// What it is called, for a script or a map to say: "hamlet".
    pub name: String,
    /// The blocks its middle stands on, by name.
    pub on: Vec<String>,
    /// How far apart, in squares: one to a patch of this side at most.
    pub spacing: u32,
    /// How likely each patch is to hold one, 0 to 1.
    pub chance: f32,
    /// How far out from its middle it reaches, in squares: all of it must be
    /// walkable, and nothing a rule grows stands within it.
    pub radius: f32,
    /// What stands in it.
    pub pieces: Vec<SitePiece>,
}

impl Default for ScatterSite {
    fn default() -> Self {
        Self {
            name: String::new(),
            on: Vec::new(),
            spacing: 96,
            chance: 0.5,
            radius: 8.0,
            pieces: Vec::new(),
        }
    }
}

/// One thing in a site.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SitePiece {
    /// The prefabs it may be, one chosen by the seed.
    pub prefabs: Vec<String>,
    /// Where it stands from the site's middle, in squares, east and north.
    pub at: [f32; 2],
    /// How likely it is to stand, 0 to 1.
    pub chance: f32,
}

impl Default for SitePiece {
    fn default() -> Self {
        Self {
            prefabs: Vec::new(),
            at: [0.0, 0.0],
            chance: 1.0,
        }
    }
}

/// The most a site may space itself.
pub const MOST_SITE_SPACING: u32 = 1024;
/// The furthest a site may reach from its middle.
pub const MOST_SITE_RADIUS: f32 = 32.0;
/// The most pieces a site may hold.
pub const MOST_PIECES: usize = 256;
/// How many places in a patch are tried before it is given up.
const TRIES: i32 = 6;
/// How many places round its edge, and half way out, a site's land is
/// sounded at for whether it can be walked.
const SOUNDINGS: usize = 16;
/// What tells a site's rolls from a rule's.
const SITE_SALT: u64 = 0x5173_5173_A5A5_0001;

/// Why a site was refused.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SiteError {
    #[error("site {0}'s `radius` must be above 0 and at most {MOST_SITE_RADIUS}")]
    Radius(usize),
    #[error(
        "site {0}'s `spacing` must be more than twice its radius, and at most {MOST_SITE_SPACING}"
    )]
    Spacing(usize),
    #[error("site {0}'s `chance` must be from 0 to 1")]
    Chance(usize),
    #[error("site {0} holds more than {MOST_PIECES} pieces")]
    Pieces(usize),
    #[error("site {0}'s piece {1} stands outside its radius, or its `chance` is not from 0 to 1")]
    Piece(usize, usize),
}

impl ScatterSite {
    /// The site, or why it would build nothing sensible; `index` names it.
    ///
    /// # Errors
    /// A radius, spacing or chance out of range, or a piece outside its radius.
    pub fn validate(&self, index: usize) -> Result<(), SiteError> {
        if !(self.radius > 0.0 && self.radius <= MOST_SITE_RADIUS) {
            return Err(SiteError::Radius(index));
        }
        #[allow(clippy::cast_precision_loss)]
        let spacing = self.spacing as f32;
        if self.spacing > MOST_SITE_SPACING || spacing <= self.radius.ceil() * 2.0 + 1.0 {
            return Err(SiteError::Spacing(index));
        }
        if !(0.0..=1.0).contains(&self.chance) {
            return Err(SiteError::Chance(index));
        }
        if self.pieces.len() > MOST_PIECES {
            return Err(SiteError::Pieces(index));
        }
        for (number, piece) in self.pieces.iter().enumerate() {
            let inside = piece.at[0].hypot(piece.at[1]) <= self.radius;
            if !inside || !(0.0..=1.0).contains(&piece.chance) {
                return Err(SiteError::Piece(index, number));
            }
        }
        Ok(())
    }

    /// Whether it makes anything at all: a site still to be filled in does not.
    fn builds(&self) -> bool {
        self.pieces
            .iter()
            .any(|piece| piece.prefabs.iter().any(|p| !p.trim().is_empty()))
    }
}

/// One place a site is built.
#[derive(Clone, Debug, PartialEq)]
pub struct Site {
    /// Which of the scatter's sites it is.
    pub site: usize,
    /// Its middle, in squares: a column and a row, as a clearing names them.
    pub middle: [f32; 2],
    /// Whether its layout is mirrored east to west.
    pub mirrored: bool,
    /// The patch it was rolled in, which its pieces are rolled from.
    patch: [i32; 2],
}

/// Every site whose middle is on the squares from `min` to `max` (inclusive,
/// column and row).
#[must_use]
pub fn sites(
    scatter: &ScatterComponent,
    ground: &VoxelGround,
    min: [i32; 2],
    max: [i32; 2],
) -> Vec<Site> {
    let mut out = Vec::new();
    for (index, site) in scatter.sites.iter().enumerate() {
        if !site.builds() || site.validate(index).is_err() {
            continue;
        }
        let Ok(spacing) = i32::try_from(site.spacing) else {
            continue;
        };
        #[allow(clippy::cast_possible_truncation)]
        let margin = site.radius.ceil() as i32;
        for patch_row in min[1].div_euclid(spacing)..=max[1].div_euclid(spacing) {
            for patch_column in min[0].div_euclid(spacing)..=max[0].div_euclid(spacing) {
                let patch = [patch_column, patch_row];
                let Some(found) = found_in(site, index, scatter.seed, ground, patch, margin) else {
                    continue;
                };
                #[allow(clippy::cast_possible_truncation)]
                let square = [
                    found.middle[0].floor() as i32,
                    found.middle[1].floor() as i32,
                ];
                let within =
                    (0..2).all(|axis| square[axis] >= min[axis] && square[axis] <= max[axis]);
                if within {
                    out.push(found);
                }
            }
        }
    }
    out
}

/// The site a patch holds, if it holds one.
fn found_in(
    site: &ScatterSite,
    index: usize,
    seed: u64,
    ground: &VoxelGround,
    patch: [i32; 2],
    margin: i32,
) -> Option<Site> {
    let spacing = i32::try_from(site.spacing).ok()?;
    let mut roll = Roll::new(
        seed ^ SITE_SALT,
        patch[0],
        patch[1],
        i32::try_from(index).ok()?,
    );
    if roll.unit() >= site.chance {
        return None;
    }
    let mirrored = roll.unit() < 0.5;
    let room = spacing - 2 * margin;
    for _ in 0..TRIES {
        let column = patch[0] * spacing + margin + roll.below(room);
        let row = patch[1] * spacing + margin + roll.below(room);
        #[allow(clippy::cast_precision_loss)]
        let middle = [column as f32 + 0.5, row as f32 + 0.5];
        if room_for(site, ground, middle, mirrored) {
            return Some(Site {
                site: index,
                middle,
                mirrored,
                patch,
            });
        }
    }
    None
}

/// Whether a site fits with its middle here: its own ground under the middle,
/// and every piece's square and the land round it walkable.
fn room_for(site: &ScatterSite, ground: &VoxelGround, middle: [f32; 2], mirrored: bool) -> bool {
    let square = |[column, row]: [f32; 2]| {
        #[allow(clippy::cast_possible_truncation)]
        let at = (column.floor() as i32, row.floor() as i32);
        at
    };
    let walkable = |at: [f32; 2]| {
        let (column, row) = square(at);
        ground
            .surface(column, row)
            .is_some_and(|(_, _, walkable)| walkable)
    };
    let (column, row) = square(middle);
    let Some((level, _, true)) = ground.surface(column, row) else {
        return false;
    };
    if !site.on.contains(&ground.block([column, level, row])) {
        return false;
    }
    let pieces = site
        .pieces
        .iter()
        .map(|piece| piece_square(middle, piece.at, mirrored));
    #[allow(clippy::cast_precision_loss)]
    let round = (0..SOUNDINGS).flat_map(|n| {
        let turn = std::f32::consts::TAU * n as f32 / SOUNDINGS as f32;
        [1.0, 0.5].map(|out| {
            [
                middle[0] + turn.cos() * site.radius * out,
                middle[1] + turn.sin() * site.radius * out,
            ]
        })
    });
    pieces.chain(round).all(walkable)
}

/// Where a piece stands, in squares, from its site's middle and its place
/// east and north of it.
fn piece_square(middle: [f32; 2], at: [f32; 2], mirrored: bool) -> [f32; 2] {
    let east = if mirrored { -at[0] } else { at[0] };
    [middle[0] + east, middle[1] - at[1]]
}

/// The key a site's piece is remembered by in `taken`: a rule number below
/// every rule's, so it is never a rule's.
fn piece_rule(site: usize, piece: usize) -> i32 {
    let code = site * MOST_PIECES + piece;
    -1 - i32::try_from(code).unwrap_or(i32::MAX - 1)
}

/// Every piece of every site that stands on the squares from `min` to `max`,
/// and has not been taken.
pub(super) fn pieces(
    scatter: &ScatterComponent,
    built: &[Site],
    min: [i32; 2],
    max: [i32; 2],
) -> Vec<Placement> {
    let mut out = Vec::new();
    for found in built {
        let site = &scatter.sites[found.site];
        let mut roll = Roll::new(
            scatter.seed ^ SITE_SALT.rotate_left(29),
            found.patch[0],
            found.patch[1],
            i32::try_from(found.site).unwrap_or(0),
        );
        for (number, piece) in site.pieces.iter().enumerate() {
            // Every piece rolls, standing or not, so one piece's chance does
            // not change what the others are.
            let stands = roll.unit() < piece.chance;
            let pick = roll.next();
            let prefabs: Vec<&String> = piece
                .prefabs
                .iter()
                .filter(|p| !p.trim().is_empty())
                .collect();
            if !stands || prefabs.is_empty() {
                continue;
            }
            let [x, y] = piece_square(found.middle, piece.at, found.mirrored);
            #[allow(clippy::cast_possible_truncation)]
            let (column, row) = (x.floor() as i32, y.floor() as i32);
            if column < min[0] || column > max[0] || row < min[1] || row > max[1] {
                continue;
            }
            let key = [column, row, piece_rule(found.site, number)];
            if scatter.taken.contains(&key) {
                continue;
            }
            let length = u64::try_from(prefabs.len()).unwrap_or(1);
            let prefab = prefabs[usize::try_from(pick % length).unwrap_or(0)].clone();
            #[allow(clippy::cast_precision_loss)]
            let offset = [x - (column as f32 + 0.5), row as f32 + 0.5 - y];
            out.push(Placement {
                key,
                prefab,
                offset,
                flipped: found.mirrored,
            });
        }
    }
    out
}

/// The furthest any of a scatter's sites reaches, in whole squares.
pub(super) fn widest(scatter: &ScatterComponent) -> i32 {
    #[allow(clippy::cast_possible_truncation)]
    let widest = scatter
        .sites
        .iter()
        .map(|site| site.radius.clamp(0.0, MOST_SITE_RADIUS).ceil() as i32)
        .max()
        .unwrap_or(0);
    widest
}

/// Whether a square's middle is inside one of these sites.
pub(super) fn in_a_site(scatter: &ScatterComponent, built: &[Site], column: i32, row: i32) -> bool {
    #[allow(clippy::cast_precision_loss)]
    let (x, y) = (column as f32 + 0.5, row as f32 + 0.5);
    built.iter().any(|found| {
        let radius = scatter.sites[found.site].radius;
        (x - found.middle[0]).hypot(y - found.middle[1]) < radius
    })
}
