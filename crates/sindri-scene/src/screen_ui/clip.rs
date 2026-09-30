//! Nested scroll rectangles shared by extraction and hit testing.
use sindri_core::{EntityId, SceneComponent, World};
use super::{ScreenExtent, ScreenRect, UiHierarchy, UiScrollComponent};
use crate::UiAnchor;

impl UiHierarchy {
    /// The intersection of ancestor scroll viewports, in overlay coordinates.
    #[must_use]
    pub fn clip_rect(&self, world: &World, entity: EntityId, extent: ScreenExtent) -> Option<ScreenRect> {
        let mut parent = world.get(entity).and_then(|d| d.parent);
        let mut clip: Option<ScreenRect> = None;
        for _ in 0..64 {
            let Some(e) = parent else { break; };
            let Some(data) = world.get(e) else { break; };
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

    /// Physical pixel scissor for an overlay draw; empty clips remain empty.
    #[must_use]
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn clip_pixels(&self, world: &World, entity: EntityId, viewport: [u32; 2]) -> Option<[u32; 4]> {
        let [w,h] = viewport;
        let rect = self.clip_rect(world, entity, ScreenExtent::new(w as f32, h as f32))?;
        let pixel = h as f32 / 2.0;
        let left = ((rect.center[0] - rect.size[0]/2.0)*pixel + w as f32/2.0).ceil().clamp(0.0,w as f32) as u32;
        let right = ((rect.center[0] + rect.size[0]/2.0)*pixel + w as f32/2.0).floor().clamp(0.0,w as f32) as u32;
        let top = (h as f32/2.0-(rect.center[1]+rect.size[1]/2.0)*pixel).ceil().clamp(0.0,h as f32) as u32;
        let bottom = (h as f32/2.0-(rect.center[1]-rect.size[1]/2.0)*pixel).floor().clamp(0.0,h as f32) as u32;
        Some([left,top,right.saturating_sub(left),bottom.saturating_sub(top)])
    }
}

pub(super) fn intersect(a: ScreenRect, b: ScreenRect) -> ScreenRect {
    let mut center = [0.0;2];
    let mut size = [0.0;2];
    for i in 0..2 {
        let low = (a.center[i]-a.size[i]/2.0).max(b.center[i]-b.size[i]/2.0);
        let high = (a.center[i]+a.size[i]/2.0).min(b.center[i]+b.size[i]/2.0);
        center[i] = (low+high)/2.0;
        size[i] = (high-low).max(0.0);
    }
    ScreenRect { center, size }
}
