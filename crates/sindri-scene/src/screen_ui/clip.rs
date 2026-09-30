//! Nested scroll rectangles shared by extraction and hit testing.
use super::{ScreenExtent, ScreenRect, UiHierarchy, UiScrollComponent};
use crate::UiAnchor;
use glam::{Mat4, Vec2, Vec3};
use sindri_core::{EntityId, SceneComponent, World};

impl UiHierarchy {
    /// The intersection of ancestor scroll viewports, in overlay coordinates.
    #[must_use]
    pub fn clip_rect(
        &self,
        world: &World,
        entity: EntityId,
        extent: ScreenExtent,
    ) -> Option<ScreenRect> {
        let mut parent = world.get(entity).and_then(|d| d.parent);
        let mut clip: Option<ScreenRect> = None;
        for _ in 0..64 {
            let Some(e) = parent else {
                break;
            };
            let Some(data) = world.get(e) else {
                break;
            };
            if data.components.contains_key(UiScrollComponent::TYPE_NAME) {
                let placed = self.placement_or(e, UiAnchor::Center);
                let origin = extent.anchor_origin(placed.anchor.unit_offset());
                let rect = ScreenRect {
                    center: [origin[0] + placed.offset.x, origin[1] + placed.offset.y],
                    size: placed.size_or(data.transform_3d.unwrap_or_default().scale_2d()),
                };
                clip = Some(clip.map_or(rect, |old| intersect(old, rect)));
            }
            parent = data.parent;
        }
        clip
    }

    /// Physical pixel scissor for an overlay drawn across the whole of a
    /// `viewport`-sized target; empty clips remain empty.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn clip_pixels(
        &self,
        world: &World,
        entity: EntityId,
        viewport: [u32; 2],
    ) -> Option<[u32; 4]> {
        let [w, h] = viewport;
        let extent = ScreenExtent::new(w as f32, h as f32);
        let half = extent.half();
        // The overlay's own projection: the extent spans clip space.
        let projection = Mat4::from_scale(Vec3::new(1.0 / half[0], 1.0 / half[1], 1.0));
        self.clip_pixels_through(world, entity, extent, projection, viewport)
    }

    /// Physical pixel scissor for an overlay of `extent` seen through
    /// `view_projection` on a `viewport`-sized target.
    ///
    /// The clip's corners are projected, so an overlay drawn onto a plane in
    /// the editor's Scene view is cut where it appears there, and a target
    /// with more pixels than points (a high-DPI one) is cut in its own pixels.
    /// A region is cut along its axis-aligned bounds: rotating a scroll
    /// region does not rotate its clip.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn clip_pixels_through(
        &self,
        world: &World,
        entity: EntityId,
        extent: ScreenExtent,
        view_projection: Mat4,
        viewport: [u32; 2],
    ) -> Option<[u32; 4]> {
        let rect = self.clip_rect(world, entity, extent)?;
        let [w, h] = [viewport[0] as f32, viewport[1] as f32];
        let mut low = Vec2::splat(f32::INFINITY);
        let mut high = Vec2::splat(f32::NEG_INFINITY);
        for corner in [[-1.0, -1.0], [1.0, -1.0], [-1.0, 1.0], [1.0, 1.0]] {
            let point = Vec3::new(
                rect.center[0] + corner[0] * rect.size[0] / 2.0,
                rect.center[1] + corner[1] * rect.size[1] / 2.0,
                0.0,
            );
            let ndc = view_projection.project_point3(point);
            let pixel = Vec2::new(f32::midpoint(ndc.x, 1.0) * w, f32::midpoint(1.0, -ndc.y) * h);
            low = low.min(pixel);
            high = high.max(pixel);
        }
        if !(low.is_finite() && high.is_finite()) {
            return Some([0, 0, 0, 0]);
        }
        let left = low.x.ceil().clamp(0.0, w) as u32;
        let top = low.y.ceil().clamp(0.0, h) as u32;
        let right = high.x.floor().clamp(0.0, w) as u32;
        let bottom = high.y.floor().clamp(0.0, h) as u32;
        Some([
            left,
            top,
            right.saturating_sub(left),
            bottom.saturating_sub(top),
        ])
    }
}

pub(super) fn intersect(a: ScreenRect, b: ScreenRect) -> ScreenRect {
    let mut center = [0.0; 2];
    let mut size = [0.0; 2];
    for i in 0..2 {
        let low = (a.center[i] - a.size[i] / 2.0).max(b.center[i] - b.size[i] / 2.0);
        let high = (a.center[i] + a.size[i] / 2.0).min(b.center[i] + b.size[i] / 2.0);
        center[i] = f32::midpoint(low, high);
        size[i] = (high - low).max(0.0);
    }
    ScreenRect { center, size }
}
