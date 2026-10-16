//! Cutting one image quad into nine, so its border keeps its size.
//!
//! A sliced image is drawn as nine quads in place of one. The corners keep the
//! size their pixels and the component's `border_scale` give them, the edges
//! stretch along their length, and the middle takes whatever is left. Each
//! piece is a quad of the parent's own space, so a sliced image turns, flips
//! and anchors exactly as the whole one would.

use glam::{Mat4, Vec3};
use sindri_render::UvRect;

use crate::SliceBorder;

/// The nine pieces of the quad `model` draws, each with the part of `rect` it
/// shows. Pieces with no area are left out, so a border of zero on a side is
/// three pieces fewer rather than three degenerate ones.
///
/// A quad smaller than its two borders squeezes them in proportion rather
/// than letting them cross: a panel shrunk past its frame stays a frame.
pub(super) fn nine_slice(
    model: Mat4,
    rect: UvRect,
    border: SliceBorder,
    scale: f32,
) -> Vec<(Mat4, UvRect)> {
    let size = [
        model.x_axis.truncate().length(),
        model.y_axis.truncate().length(),
    ];
    #[allow(clippy::cast_precision_loss)]
    let world = border.pixels.map(|pixels| pixels as f32 * scale.max(0.0));
    let [left, right] = fit([world[0], world[2]], size[0]);
    let [top, bottom] = fit([world[1], world[3]], size[1]);
    // Every edge in the quad's own -0.5..0.5 space, and in the same space
    // across the rect: the convention `UvRect::part` reads.
    let xs = [-0.5, left - 0.5, 0.5 - right, 0.5];
    let ys = [-0.5, bottom - 0.5, 0.5 - top, 0.5];
    let us = [
        -0.5,
        border.uv[0] / rect.width() - 0.5,
        0.5 - border.uv[2] / rect.width(),
        0.5,
    ];
    let vs = [
        -0.5,
        border.uv[3] / rect.height() - 0.5,
        0.5 - border.uv[1] / rect.height(),
        0.5,
    ];
    let mut pieces = Vec::with_capacity(9);
    for row in 0..3 {
        for column in 0..3 {
            let width = xs[column + 1] - xs[column];
            let height = ys[row + 1] - ys[row];
            if width <= f32::EPSILON || height <= f32::EPSILON {
                continue;
            }
            let Some(uv) = rect.part(
                [
                    f32::midpoint(us[column], us[column + 1]),
                    f32::midpoint(vs[row], vs[row + 1]),
                ],
                [us[column + 1] - us[column], vs[row + 1] - vs[row]],
            ) else {
                continue;
            };
            let centre = Vec3::new(
                f32::midpoint(xs[column], xs[column + 1]),
                f32::midpoint(ys[row], ys[row + 1]),
                0.0,
            );
            let piece = model
                * Mat4::from_translation(centre)
                * Mat4::from_scale(Vec3::new(width, height, 1.0));
            pieces.push((piece, uv));
        }
    }
    pieces
}

/// Two borders as fractions of a side `length` long, squeezed together when
/// they would not fit.
fn fit(borders: [f32; 2], length: f32) -> [f32; 2] {
    if length <= 0.0 {
        return [0.0, 0.0];
    }
    let total = borders[0] + borders[1];
    let squeeze = if total > length { length / total } else { 1.0 };
    borders.map(|border| border * squeeze / length)
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Vec3};
    use sindri_render::UvRect;

    use super::nine_slice;
    use crate::SliceBorder;

    fn close(left: f32, right: f32) -> bool {
        (left - right).abs() < 1.0e-5
    }

    /// A four-by-two panel with a four-pixel frame at sixteen pixels a unit:
    /// the corners are a quarter unit square however wide the panel is, and
    /// draw the corner of the art.
    #[test]
    fn the_corners_keep_their_size() {
        let model = Mat4::from_scale(Vec3::new(4.0, 2.0, 1.0));
        let border = SliceBorder {
            uv: [0.25; 4],
            pixels: [4; 4],
        };
        let pieces = nine_slice(model, UvRect::FULL, border, 1.0 / 16.0);
        assert_eq!(pieces.len(), 9);
        let (corner, uv) = pieces[0];
        assert!(close(corner.x_axis.x, 0.25) && close(corner.y_axis.y, 0.25));
        assert!(close(corner.w_axis.x, -1.875) && close(corner.w_axis.y, -0.875));
        // The bottom-left piece shows the bottom-left of the art, which is
        // the far end of the texture vertically.
        assert!(close(uv.x(), 0.0) && close(uv.y(), 0.75) && close(uv.width(), 0.25));
        let (middle, uv) = pieces[4];
        assert!(close(middle.x_axis.x, 3.5) && close(middle.y_axis.y, 1.5));
        assert!(close(uv.x(), 0.25) && close(uv.width(), 0.5));
    }

    /// Shrunk past its frame, a panel squeezes the frame rather than turning
    /// it inside out, and the middle disappears.
    #[test]
    fn a_panel_smaller_than_its_frame_squeezes_it() {
        let model = Mat4::from_scale(Vec3::new(0.25, 0.25, 1.0));
        let border = SliceBorder {
            uv: [0.25; 4],
            pixels: [4; 4],
        };
        let pieces = nine_slice(model, UvRect::FULL, border, 1.0 / 16.0);
        assert_eq!(pieces.len(), 4, "only the corners are left");
        assert!(pieces.iter().all(|(piece, _)| close(piece.x_axis.x, 0.125)));
    }

    /// A border of nothing on two sides leaves a three-piece strip: a bar with
    /// end caps.
    #[test]
    fn a_border_on_two_sides_is_three_pieces() {
        let model = Mat4::from_scale(Vec3::new(3.0, 1.0, 1.0));
        let border = SliceBorder {
            uv: [0.25, 0.0, 0.25, 0.0],
            pixels: [4, 0, 4, 0],
        };
        assert_eq!(nine_slice(model, UvRect::FULL, border, 1.0 / 16.0).len(), 3);
    }
}
