//! A sprite mask: a picture whose shape decides where a batch draws.
//!
//! The mask is the batch's, not each sprite's, which is what keeps it cheap:
//! one texture and one matrix in the batch uniform rather than another sixty
//! bytes in every instance. Sprites masked differently are different batches.

use glam::Mat4;

use crate::{TextureId, UvRect};

/// Where a batch may draw: inside or outside the opaque part of a picture
/// placed in the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteMask {
    /// The picture whose alpha is the mask's shape.
    pub texture: TextureId,
    /// The part of `texture` the mask shows.
    pub uv: UvRect,
    /// From the world into the mask quad's own -0.5..0.5 space: the inverse
    /// of the matrix a sprite drawing the mask would have.
    pub from_world: Mat4,
    /// Draw only outside the shape rather than only inside it.
    pub outside: bool,
}

impl SpriteMask {
    /// The lane of the batch uniform's flags that says how to mask: 0 for
    /// none, 1 inside, 2 outside.
    pub(super) fn mode(mask: Option<&Self>) -> f32 {
        match mask {
            None => 0.0,
            Some(mask) if mask.outside => 2.0,
            Some(_) => 1.0,
        }
    }
}
