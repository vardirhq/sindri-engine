//! Read-only guides for the selected camera's authored orbit and current pose.

use std::f32::consts::TAU;

use eframe::egui::{Color32, FontId, Painter, Rect, Stroke, Vec2};
use glam::{Mat4, Vec3};
use sindri_core::{EntityId, SceneComponent, World};
use sindri_scene::{CameraComponent, CameraOrbitComponent};

use crate::ui::theme::{color, text};

use super::super::projection::{project_point, project_segment};
use super::EditorApp;

#[derive(Debug)]
struct Guide {
    focus: Vec3,
    desired: Vec3,
    current: Vec3,
    radius: f32,
}

impl Guide {
    fn from_world(world: &World, camera: EntityId) -> Option<Self> {
        if !world.is_active(camera) {
            return None;
        }
        let data = world.get(camera)?;
        serde_json::from_value::<CameraComponent>(
            data.components.get(CameraComponent::TYPE_NAME)?.clone(),
        )
        .ok()?;
        let orbit: CameraOrbitComponent = serde_json::from_value(
            data.components
                .get(CameraOrbitComponent::TYPE_NAME)?
                .clone(),
        )
        .ok()?;
        if !orbit.is_valid() {
            return None;
        }
        let target = world.entity_for_source_id(&orbit.target)?;
        if target == camera
            || !world.is_active(target)
            || world.check_set_parent(camera, Some(target)).is_err()
        {
            return None;
        }
        // Offset and angles use world axes, including on rotated/scaled parents.
        let focus = Vec3::from_array(world.world_transform(target)?.position)
            + Vec3::from_array(orbit.offset);
        let (yaw_sin, yaw_cos) = orbit.yaw.sin_cos();
        let (pitch_sin, pitch_cos) = orbit.pitch.sin_cos();
        let direction = Vec3::new(yaw_sin * pitch_cos, pitch_sin, yaw_cos * pitch_cos);
        let desired = focus + direction * orbit.distance;
        let current = Vec3::from_array(world.world_transform(camera)?.position);
        let radius = orbit.distance * pitch_cos;
        (focus.is_finite() && desired.is_finite() && current.is_finite()).then_some(Self {
            focus,
            desired,
            current,
            radius,
        })
    }

    /// The yaw circle at the requested pitch and distance, in world space.
    fn yaw_segments(&self) -> impl Iterator<Item = [Vec3; 2]> + '_ {
        let centre = Vec3::new(self.focus.x, self.desired.y, self.focus.z);
        let point = move |step: u16| {
            let (sin, cos) = (TAU * f32::from(step) / 48.0).sin_cos();
            centre + Vec3::new(sin * self.radius, 0.0, cos * self.radius)
        };
        (0..48).map(move |step| [point(step), point(step + 1)])
    }

    fn paint(&self, painter: &Painter, rect: Rect, view_projection: Mat4) {
        let line = |points: [Vec3; 2], stroke| {
            if let Some(projected) = project_segment(rect, view_projection, points[0], points[1]) {
                painter.line_segment(projected, stroke);
            }
        };
        for segment in self.yaw_segments() {
            line(segment, Stroke::new(1.0, color::FORGE_DIM));
        }
        line(
            [self.focus, self.desired],
            Stroke::new(2.0, color::FORGE_BRIGHT),
        );
        let marker = |position, label: &str, tint, above| {
            if let Some(point) = project_point(rect, view_projection, position) {
                painter.circle_stroke(point, 5.0, Stroke::new(2.0, tint));
                let galley = painter.layout_no_wrap(
                    label.to_owned(),
                    FontId::proportional(text::NOTE),
                    tint,
                );
                let size = galley.size();
                let origin = point + Vec2::new(8.0, if above { -8.0 - size.y } else { 8.0 });
                painter.rect_filled(
                    Rect::from_min_size(origin - Vec2::splat(2.0), size + Vec2::splat(4.0)),
                    2.0,
                    Color32::from_black_alpha(180),
                );
                painter.galley(origin, galley, tint);
            }
        };
        marker(self.focus, "Orbit focus", color::FORGE_BRIGHT, true);
        marker(self.desired, "Requested orbit", color::FORGE_BRIGHT, true);
        if self.current.distance(self.desired) > 0.05 {
            line([self.focus, self.current], Stroke::new(1.5, color::AXIS_Z));
            marker(self.current, "Camera now", color::AXIS_Z, false);
        }
    }
}

pub(super) fn paint_selected(app: &EditorApp, painter: &Painter, rect: Rect) {
    let aspect = rect.width() / rect.height().max(1.0);
    let Ok(Some(view)) =
        app.scene
            .world_camera_for_viewport(&app.world, aspect, app.scene_camera())
    else {
        return;
    };
    // Guides never participate in camera picking or mutate the scene. They also
    // survive when the current camera marker lies behind the Scene view's eye.
    for entity in app.selection.all() {
        if let Some(guide) = Guide::from_world(&app.world, *entity) {
            guide.paint(painter, rect, view.view_projection);
        }
    }
}

#[cfg(test)]
mod tests;
