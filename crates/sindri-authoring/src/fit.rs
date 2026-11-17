//! A collider fitted to the picture its object draws.
//!
//! Drawing a collider by hand around every tree, crate and rock is the chore
//! that makes scenes wrong: the art is redrawn, the collider is not, and the
//! player walks into air or through a trunk. The picture already says where
//! the thing is, so this reads it. Two fits, because a picture means two
//! different things depending on how it is seen:
//!
//! - [`FitTo::Outline`] is everything drawn: a box around the visible pixels,
//!   for side-view games, where a crate blocks wherever it is drawn.
//! - [`FitTo::Footing`] is where the thing meets the ground: the lowest band of
//!   what is drawn, laid down as a footprint shallower than it is wide, for a
//!   picture standing up in a world seen from above at a slant. A tree is
//!   blocked by its trunk, so whoever walks behind it passes under its crown.
//!
//! The light occluder, when the object has one, is given the same outline, so
//! what stops a body also casts the shadow. Only the first piece of a compound
//! collider is fitted; the others are the author's.
//!
//! Proven in Lamplighter, whose generated wilds were fitted this way by its
//! art script before the editors could do it.

use std::collections::BTreeMap;
use std::f32::consts::TAU;
use std::f64::consts::FRAC_PI_2;
use std::path::Path;

use serde_json::{Value, json};
use sindri_core::{SpriteAnchor, SpriteRef, SpriteSheetDocument};

const SPRITE: &str = "sindri.sprite";
const COLLIDER: &str = "sindri.physics2d.collider";
const OCCLUDER: &str = "sindri.light2d.occluder";

/// How much of the drawn height, from the bottom, is a footing; the fewest rows
/// it is; how much of that band a column must be drawn in to count, so a stray
/// blade of grass does not widen it; and how deep it is for its width.
const BASE: f32 = 0.14;
const FEWEST_ROWS: u32 = 3;
const SOLID: f32 = 0.5;
const DEPTH: f32 = 0.45;
/// Below this alpha a pixel is not drawn: antialiased fringes and shadows
/// baked at a whisper do not widen a body.
const DRAWN: u8 = 32;
/// Points in a footing's occluder outline.
const OUTLINE_POINTS: u16 = 12;

/// Which reading of the picture a collider is fitted to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitTo {
    /// A box around everything drawn.
    Outline,
    /// A footprint at the base of what is drawn.
    Footing,
}

/// One frame of a picture: its alpha, row by row from the top.
#[derive(Clone, Debug)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub alpha: Vec<u8>,
}

impl Frame {
    fn drawn(&self, x: u32, y: u32) -> bool {
        self.alpha[(y * self.width + x) as usize] >= DRAWN
    }

    /// The drawn pixels' bounds as (left, top, right, bottom), inclusive.
    fn bounds(&self) -> Option<[u32; 4]> {
        let mut found: Option<[u32; 4]> = None;
        for y in 0..self.height {
            for x in 0..self.width {
                if self.drawn(x, y) {
                    let bounds = found.get_or_insert([x, y, x, y]);
                    bounds[0] = bounds[0].min(x);
                    bounds[1] = bounds[1].min(y);
                    bounds[2] = bounds[2].max(x);
                    bounds[3] = bounds[3].max(y);
                }
            }
        }
        found
    }
}

/// Where a frame is drawn in its object's own space: the sprite's size and
/// offset, and the anchor its sheet gives it.
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub size: [f32; 2],
    pub offset: [f32; 2],
    pub anchor: SpriteAnchor,
}

impl Placement {
    /// A point on the frame, in pixels from its top-left, in the object's space.
    #[allow(clippy::cast_precision_loss)]
    fn place(self, frame: &Frame, pixel: [f32; 2]) -> [f32; 2] {
        let [anchor_x, anchor_y] = self.anchor.offset();
        let quad = [
            pixel[0] / frame.width as f32 - 0.5 + anchor_x,
            0.5 - pixel[1] / frame.height as f32 + anchor_y,
        ];
        [
            self.offset[0] + quad[0] * self.size[0],
            self.offset[1] + quad[1] * self.size[1],
        ]
    }
}

/// A collider shape fitted to a frame, in the object's own space.
#[derive(Clone, Debug, PartialEq)]
pub struct Fitted {
    /// The shape as a collider stores it: `{"shape": "box", ...}`.
    pub shape: Value,
    pub offset: [f64; 2],
    pub rotation: f64,
    /// The same outline as a closed polygon, for an occluder.
    pub outline: Vec<[f64; 2]>,
}

/// Fits a shape to `frame` drawn at `placement`, or `None` when nothing in it
/// is drawn.
#[must_use]
pub fn fit(frame: &Frame, placement: &Placement, to: FitTo) -> Option<Fitted> {
    let [left, top, right, bottom] = frame.bounds()?;
    #[allow(clippy::cast_precision_loss)]
    let corner = |x: u32, y: u32| placement.place(frame, [x as f32, y as f32]);
    match to {
        FitTo::Outline => {
            let low = corner(left, bottom + 1);
            let high = corner(right + 1, top);
            Some(boxed(low, high))
        }
        FitTo::Footing => {
            let drawn_rows = bottom - top + 1;
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            let rows = FEWEST_ROWS
                .max((drawn_rows as f32 * BASE).round() as u32)
                .min(drawn_rows);
            let band = (bottom + 1 - rows)..=bottom;
            let solid = |x: u32, share: f32| {
                let count = band.clone().filter(|&y| frame.drawn(x, y)).count();
                #[allow(clippy::cast_precision_loss)]
                let enough = count as f32 >= share * rows as f32;
                count > 0 && enough
            };
            // The columns solidly drawn across the band, or, for a picture too
            // ragged to have any, every column drawn in it at all.
            let span = |share: f32| {
                let first = (0..frame.width).find(|&x| solid(x, share))?;
                let last = (0..frame.width).rev().find(|&x| solid(x, share))?;
                Some((first, last))
            };
            let (low, high) = span(SOLID).or_else(|| span(0.0))?;
            let foot_left = corner(low, bottom + 1);
            let foot_right = corner(high + 1, bottom + 1 - rows);
            Some(footing(foot_left, foot_right))
        }
    }
}

/// A box with corners `low` and `high`, either way round.
fn boxed(low: [f32; 2], high: [f32; 2]) -> Fitted {
    let centre = [low[0].midpoint(high[0]), low[1].midpoint(high[1])];
    let half = [
        (high[0] - low[0]).abs() / 2.0,
        (high[1] - low[1]).abs() / 2.0,
    ];
    Fitted {
        shape: json!({"shape": "box", "half_extents": [round(half[0]), round(half[1])]}),
        offset: [round(centre[0]), round(centre[1])],
        rotation: 0.0,
        outline: [
            [centre[0] - half[0], centre[1] - half[1]],
            [centre[0] + half[0], centre[1] - half[1]],
            [centre[0] + half[0], centre[1] + half[1]],
            [centre[0] - half[0], centre[1] + half[1]],
        ]
        .into_iter()
        .map(|[x, y]| [round(x), round(y)])
        .collect(),
    }
}

/// A footprint across the band from `left` to `right`: a capsule on its side,
/// or a circle when it is too narrow to be one.
fn footing(left: [f32; 2], right: [f32; 2]) -> Fitted {
    let reach = (right[0] - left[0]).abs() / 2.0;
    let radius = (reach * DEPTH).max(f32::EPSILON);
    let centre = [left[0].midpoint(right[0]), left[1].midpoint(right[1])];
    let (shape, rotation) = if reach - radius < reach * 0.05 {
        (json!({"shape": "circle", "radius": round(reach)}), 0.0)
    } else {
        // A capsule stands along its own y; laid on its side, it runs across.
        (
            json!({"shape": "capsule", "half_height": round(reach - radius), "radius": round(radius)}),
            (FRAC_PI_2 * 1e6).round() / 1e6,
        )
    };
    let outline = (0..OUTLINE_POINTS)
        .map(|n| {
            let angle = TAU * f32::from(n) / f32::from(OUTLINE_POINTS);
            [
                round(centre[0] + reach * angle.cos()),
                round(centre[1] + radius * angle.sin()),
            ]
        })
        .collect();
    Fitted {
        shape,
        offset: [round(centre[0]), round(centre[1])],
        rotation,
        outline,
    }
}

/// Five places, and widened first, so a fitted scene reads `0.4` rather than
/// the float noise of `0.4000000059604645`.
fn round(value: f32) -> f64 {
    (f64::from(value) * 100_000.0).round() / 100_000.0
}

/// Why a collider could not be fitted.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FitError {
    #[error("this object draws no sprite to fit to")]
    NoSprite,
    #[error("this object has no 2D collider")]
    NoCollider,
    #[error("the sprite does not read: {0}")]
    BadSprite(String),
    #[error("the picture {0} could not be opened: {1}")]
    Picture(String, String),
    #[error("the sheet beside {0} does not name {1}")]
    UnknownFrame(String, String),
    #[error("nothing in the picture is drawn")]
    Empty,
}

/// Reads the frame a sprite draws from the project's assets, and the anchor
/// its sheet gives it.
///
/// # Errors
/// When the image cannot be read, or its sheet does not name the frame.
pub fn load_frame(
    assets_root: &Path,
    reference: &SpriteRef,
) -> Result<(Frame, SpriteAnchor), FitError> {
    let texture = reference.texture();
    let image = image::open(assets_root.join(texture))
        .map_err(|error| FitError::Picture(texture.to_owned(), error.to_string()))?
        .to_rgba8();
    let sheet = reference
        .sheet()
        .and_then(|id| std::fs::read_to_string(assets_root.join(id.as_str())).ok())
        .and_then(|json| SpriteSheetDocument::from_json(&json).ok());
    let anchor = sheet
        .as_ref()
        .and_then(|sheet| sheet.anchor)
        .unwrap_or_default();
    let rect = match reference.sprite() {
        Some(name) => sheet
            .as_ref()
            .and_then(|sheet| sheet.rects().ok())
            .and_then(|rects| rects.get(name).copied())
            .ok_or_else(|| FitError::UnknownFrame(texture.to_owned(), name.to_owned()))?,
        None => [0.0, 0.0, 1.0, 1.0],
    };
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let pixel = |fraction: f32, of: u32| ((fraction * of as f32).round() as u32).min(of);
    let (x, y) = (
        pixel(rect[0], image.width()),
        pixel(rect[1], image.height()),
    );
    let width = pixel(rect[2], image.width()).min(image.width() - x);
    let height = pixel(rect[3], image.height()).min(image.height() - y);
    let mut alpha = Vec::with_capacity((width * height) as usize);
    for row in y..y + height {
        for column in x..x + width {
            alpha.push(image.get_pixel(column, row)[3]);
        }
    }
    Ok((
        Frame {
            width,
            height,
            alpha,
        },
        anchor,
    ))
}

/// Refits the 2D collider on an object's `components` to the sprite it draws,
/// and its light occluder, if it has one, to the same outline.
///
/// # Errors
/// When the object has no sprite or collider, or its picture cannot be read or
/// draws nothing.
pub fn fit_components(
    assets_root: &Path,
    components: &mut BTreeMap<String, Value>,
    to: FitTo,
) -> Result<(), FitError> {
    let sprite = components.get(SPRITE).ok_or(FitError::NoSprite)?;
    let sprite: sindri_scene::SpriteComponent = serde_json::from_value(sprite.clone())
        .map_err(|error| FitError::BadSprite(error.to_string()))?;
    if !components.contains_key(COLLIDER) {
        return Err(FitError::NoCollider);
    }
    let reference = sprite
        .reference()
        .map_err(|error| FitError::BadSprite(error.to_string()))?;
    let (frame, anchor) = load_frame(assets_root, &reference)?;
    let placement = Placement {
        size: sprite.size,
        offset: sprite.offset,
        anchor,
    };
    let fitted = fit(&frame, &placement, to).ok_or(FitError::Empty)?;
    let collider = components.get_mut(COLLIDER).ok_or(FitError::NoCollider)?;
    apply(collider, &fitted);
    if let Some(occluder) = components.get_mut(OCCLUDER)
        && let Some(occluder) = occluder.as_object_mut()
    {
        occluder.insert("shape".to_owned(), json!("polygon"));
        occluder.insert("points".to_owned(), json!(fitted.outline));
    }
    Ok(())
}

/// Writes `fitted` into a collider payload: its first piece when it has pieces.
fn apply(collider: &mut Value, fitted: &Fitted) {
    let piece = match collider.get_mut("pieces").and_then(Value::as_array_mut) {
        Some(pieces) if !pieces.is_empty() => &mut pieces[0],
        _ => collider,
    };
    if let Some(piece) = piece.as_object_mut() {
        piece.insert("shape".to_owned(), fitted.shape.clone());
        piece.insert("offset".to_owned(), json!(fitted.offset));
        piece.insert("rotation".to_owned(), json!(fitted.rotation));
    }
}

#[cfg(test)]
mod tests;
