//! Conservative bounds tests shared by cached scene geometry.

use glam::{Mat4, Vec3};

/// Whether any part of an axis-aligned box could be inside the camera frustum.
///
/// A box is rejected only when all eight corners lie beyond the same clip
/// plane. That can retain a box which does not really intersect the frustum,
/// but it cannot cut a visible piece out of the world.
pub(super) fn aabb_in_view(min: Vec3, max: Vec3, view_projection: Mat4) -> bool {
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, max.y, max.z),
    ];
    let clip = corners.map(|corner| view_projection * corner.extend(1.0));
    !(clip.iter().all(|point| point.x < -point.w)
        || clip.iter().all(|point| point.x > point.w)
        || clip.iter().all(|point| point.y < -point.w)
        || clip.iter().all(|point| point.y > point.w)
        || clip.iter().all(|point| point.z < 0.0)
        || clip.iter().all(|point| point.z > point.w))
}

/// The view sprites are culled against: the world camera's, when it is
/// orthographic.
///
/// A 2D game's camera, where what is outside the view is outside the screen
/// rectangle and nothing more. Under a perspective camera every sprite is
/// still handed to the renderer, as before, because 2.5D scenes billboard and
/// lean sprites towards an eye whose frustum a flat quad test would misjudge
/// near its edges.
pub(super) fn culling_view(view_projection: Mat4) -> Option<Mat4> {
    let w = view_projection.row(3);
    let orthographic = w.x.abs() < 1.0e-6 && w.y.abs() < 1.0e-6 && w.z.abs() < 1.0e-6;
    orthographic.then_some(view_projection)
}

/// Whether any part of a sprite's unit quad, placed by `model`, could reach
/// the view.
///
/// The quad is what the sprite pipeline draws, `[-0.5, 0.5]` square, with no
/// vertex displacement, so a quad wholly beyond one side of the view is a
/// sprite no pixel shows. Only the four sides are tested: a sprite in front
/// of or behind the camera's range is left to the depth test, as it was.
/// A corner behind a perspective eye keeps the quad, since its side cannot be
/// read off the clip coordinates.
pub(super) fn quad_in_view(model: Mat4, view_projection: Mat4) -> bool {
    let clip = [[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]]
        .map(|[x, y]| view_projection * model * glam::Vec4::new(x, y, 0.0, 1.0));
    rect_in_clip(&clip)
}

/// Whether a rectangle on the plane `z`, from `min` to `max`, could reach the
/// view: [`quad_in_view`] for a world-space rectangle such as a light's reach.
pub(super) fn rect_in_view(min: [f32; 2], max: [f32; 2], z: f32, view_projection: Mat4) -> bool {
    let clip = [
        [min[0], min[1]],
        [max[0], min[1]],
        [max[0], max[1]],
        [min[0], max[1]],
    ]
    .map(|[x, y]| view_projection * glam::Vec4::new(x, y, z, 1.0));
    rect_in_clip(&clip)
}

fn rect_in_clip(clip: &[glam::Vec4; 4]) -> bool {
    if clip.iter().any(|point| point.w <= 0.0) {
        return true;
    }
    !(clip.iter().all(|point| point.x < -point.w)
        || clip.iter().all(|point| point.x > point.w)
        || clip.iter().all(|point| point.y < -point.w)
        || clip.iter().all(|point| point.y > point.w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_only_boxes_wholly_beyond_one_clip_plane() {
        assert!(aabb_in_view(
            Vec3::new(-0.5, -0.5, 0.25),
            Vec3::new(0.5, 0.5, 0.75),
            Mat4::IDENTITY,
        ));
        assert!(!aabb_in_view(
            Vec3::new(2.0, -0.5, 0.25),
            Vec3::new(3.0, 0.5, 0.75),
            Mat4::IDENTITY,
        ));
        assert!(aabb_in_view(
            Vec3::new(0.5, -0.5, 0.25),
            Vec3::new(2.0, 0.5, 0.75),
            Mat4::IDENTITY,
        ));
    }

    #[test]
    fn a_quad_is_kept_while_any_of_it_is_on_screen() {
        let view = Mat4::IDENTITY;
        assert!(quad_in_view(
            Mat4::from_translation(Vec3::new(0.0, 0.0, 0.5)),
            view
        ));
        // Half off the right edge: still drawn.
        assert!(quad_in_view(
            Mat4::from_translation(Vec3::new(1.4, 0.0, 0.5)),
            view
        ));
        // Wholly beyond it, or above the top.
        assert!(!quad_in_view(
            Mat4::from_translation(Vec3::new(1.6, 0.0, 0.5)),
            view
        ));
        assert!(!quad_in_view(
            Mat4::from_translation(Vec3::new(0.0, 2.0, 0.5)),
            view
        ));
        // Large enough to straddle the view from outside.
        assert!(quad_in_view(
            Mat4::from_translation(Vec3::new(3.0, 0.0, 0.5)) * Mat4::from_scale(Vec3::splat(8.0)),
            view
        ));
    }

    #[test]
    fn a_rect_is_kept_while_it_overlaps_the_view() {
        let view = Mat4::IDENTITY;
        assert!(rect_in_view([0.9, -0.2], [3.0, 0.2], 0.5, view));
        assert!(!rect_in_view([1.1, -0.2], [3.0, 0.2], 0.5, view));
    }
}
