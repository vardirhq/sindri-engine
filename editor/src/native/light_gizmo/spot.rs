//! World-space spotlight cones, clipped through the same seam as light arrows.
use super::{
    EntityId, LightComponent, LightVisual, Mat4, Rect, Vec3, light_direction, light_visual,
    project_segment,
};
use sindri_core::Transform3D;

pub(super) fn visual(
    entity: EntityId,
    transform: Transform3D,
    light: LightComponent,
    rect: Rect,
    view_projection: Mat4,
    selected: bool,
) -> Option<LightVisual> {
    let position = Vec3::from_array(transform.position);
    let direction = light_direction(transform);
    let mut visual = light_visual(entity, position, direction, rect, view_projection, false)?;
    if selected {
        let a = direction.any_orthonormal_vector();
        let b = direction.cross(a);
        for angle in [light.inner_angle, light.outer_angle] {
            // Range is radial distance, so the guide meets the spherical cutoff.
            let center = position + direction * (light.range * angle.cos());
            let radius = light.range * angle.sin();
            for step in 0..32_u8 {
                let from = std::f32::consts::TAU * f32::from(step) / 32.0;
                let to = std::f32::consts::TAU * f32::from(step + 1) / 32.0;
                let edge = center + radius * (a * from.cos() + b * from.sin());
                let next = center + radius * (a * to.cos() + b * to.sin());
                if let Some(segment) = project_segment(rect, view_projection, edge, next) {
                    visual.range_segments.push(segment);
                }
                if step % 8 == 0
                    && let Some(segment) = project_segment(rect, view_projection, position, edge)
                {
                    visual.range_segments.push(segment);
                }
            }
        }
    }
    Some(visual)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{self, Pos2};
    use sindri_scene::LightKind;
    #[test]
    fn cones_follow_rotation_ignore_scale_and_hide_when_unselected() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 400.0));
        let camera = sindri_render::perspective_projection(1.0, 1.0, 0.1, 100.0)
            * sindri_render::look_at(Vec3::Z * 10.0, Vec3::ZERO, Vec3::Y);
        let entity = sindri_core::World::default().spawn(sindri_core::EntityData::default());
        let light = LightComponent {
            kind: LightKind::Spot,
            range: 1.0,
            ..Default::default()
        };
        let transform = Transform3D {
            rotation: glam::Quat::from_rotation_y(0.4).to_array(),
            ..Default::default()
        };
        let selected = visual(entity, transform, light, rect, camera, true).unwrap();
        assert_eq!(selected.range_segments.len(), 72);
        assert!(selected.arrow.is_some() && selected.trail.is_none());
        let scaled = visual(
            entity,
            Transform3D {
                scale: [5.0; 3],
                ..transform
            },
            light,
            rect,
            camera,
            true,
        )
        .unwrap();
        assert_eq!(scaled.range_segments, selected.range_segments);
        assert!(
            visual(entity, transform, light, rect, camera, false)
                .unwrap()
                .range_segments
                .is_empty()
        );
        assert!(
            visual(
                entity,
                Transform3D {
                    position: [0.0, 0.0, 11.0],
                    ..transform
                },
                light,
                rect,
                camera,
                true
            )
            .is_none()
        );
    }
}
