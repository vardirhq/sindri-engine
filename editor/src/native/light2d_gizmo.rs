//! 2D lights and occluders as the Scene view draws them: each light's reach
//! as a circle, each occluder's outline, and handles on the selected one to
//! size it by dragging.
//!
//! A light's radius is a distance, and a distance typed into a field is a
//! guess at how far it reaches until the scene is lit. Drawn, it is where the
//! dark starts. A drag is one undo step, as a collider's is.

use eframe::egui::{self, Color32, LayerId, Order, Pos2, Rect, Response, Stroke};
use glam::{Mat4, Vec3};
use serde_json::Value;
use sindri_core::{CommandBuffer, EntityId, SceneComponent, WorldCommand};
use sindri_scene::{Light2dComponent, Light2dOccluderComponent, OccluderShape};

use super::EditorApp;
use super::projection::{project_point, project_segment, unproject_to_plane};

const LIGHT_LAYER: &str = "sindri-light2d-overlay";
const LIGHT_DRAG: &str = "sindri-light2d-drag";

const LIGHT_AMBER: Color32 = Color32::from_rgb(255, 200, 90);
const OCCLUDER_VIOLET: Color32 = Color32::from_rgb(190, 140, 255);

const HANDLE_REACH: f32 = 8.0;
const HANDLE_SIZE: f32 = 7.0;
const SMALLEST: f32 = 0.02;
const CIRCLE_SEGMENTS: usize = 48;

/// What a handle sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Grip {
    /// A light's radius.
    Radius,
    /// One half of a box occluder's size, along axis 0 (X) or 1 (Y). The box
    /// is centred on its entity, so it grows evenly either side.
    Size(usize),
}

#[derive(Clone, Copy, Debug)]
struct LightDrag {
    entity: EntityId,
    grip: Grip,
}

/// The size a handle dragged to `local`, a point in the entity's own plane,
/// asks for.
fn dragged(grip: Grip, local: [f32; 2]) -> f32 {
    match grip {
        Grip::Radius => local[0].hypot(local[1]).max(SMALLEST),
        Grip::Size(axis) => (local[axis].abs() * 2.0).max(SMALLEST),
    }
}

fn circle(center: Vec3, radius: f32) -> Vec<Vec3> {
    #[allow(clippy::cast_precision_loss)]
    (0..CIRCLE_SEGMENTS)
        .map(|index| {
            let angle = index as f32 / CIRCLE_SEGMENTS as f32 * std::f32::consts::TAU;
            center + Vec3::new(angle.cos() * radius, angle.sin() * radius, 0.0)
        })
        .collect()
}

fn paint_loop(
    painter: &egui::Painter,
    rect: Rect,
    view_projection: Mat4,
    points: &[Vec3],
    stroke: Stroke,
) {
    for (index, start) in points.iter().enumerate() {
        let end = points[(index + 1) % points.len()];
        if let Some(segment) = project_segment(rect, view_projection, *start, end) {
            painter.line_segment(segment, stroke);
        }
    }
}

impl EditorApp {
    /// Draws every 2D light's reach and every occluder's outline, and lets
    /// the selected one's handle be dragged.
    ///
    /// Returns whether a handle has the pointer, so the move gizmo and the
    /// camera leave the drag alone.
    pub(super) fn light2d_overlay(
        &mut self,
        context: &egui::Context,
        response: &Response,
        blocked: bool,
    ) -> bool {
        let rect = response.rect;
        let aspect = rect.width() / rect.height().max(1.0);
        let Some(camera) = self
            .scene
            .world_camera_for_viewport(&self.world, aspect, self.scene_camera())
            .ok()
            .flatten()
        else {
            return false;
        };
        let view_projection = camera.view_projection;
        let painter = context
            .layer_painter(LayerId::new(Order::Background, egui::Id::new(LIGHT_LAYER)))
            .with_clip_rect(rect);
        let components = self.scene.components();
        let lights = components
            .query::<Light2dComponent>(&self.world)
            .unwrap_or_default();
        let occluders = components
            .query::<Light2dOccluderComponent>(&self.world)
            .unwrap_or_default();
        let mut handles: Vec<(EntityId, Grip, Pos2, Mat4)> = Vec::new();
        for (entity, light) in &lights {
            let matrix = self.matrix_of(*entity);
            let center = matrix.transform_point3(Vec3::ZERO);
            let selected = self.selection.contains(*entity);
            let stroke = Stroke::new(
                if selected { 1.75 } else { 1.0 },
                if selected {
                    LIGHT_AMBER
                } else {
                    LIGHT_AMBER.gamma_multiply(0.5)
                },
            );
            paint_loop(
                &painter,
                rect,
                view_projection,
                &circle(center, light.radius),
                stroke,
            );
            if selected
                && let Some(at) = project_point(
                    rect,
                    view_projection,
                    center + Vec3::new(light.radius, 0.0, 0.0),
                )
            {
                handles.push((*entity, Grip::Radius, at, Mat4::from_translation(center)));
            }
        }
        for (entity, occluder) in &occluders {
            if occluder.shape == OccluderShape::Tilemap {
                continue;
            }
            let Ok(edges) = occluder.local_edges() else {
                continue;
            };
            let matrix = self.matrix_of(*entity);
            let selected = self.selection.contains(*entity);
            let stroke = Stroke::new(
                if selected { 1.75 } else { 1.0 },
                if selected {
                    OCCLUDER_VIOLET
                } else {
                    OCCLUDER_VIOLET.gamma_multiply(0.6)
                },
            );
            for [a, b] in edges {
                let [a, b] = [a, b].map(|[x, y]| matrix.transform_point3(Vec3::new(x, y, 0.0)));
                if let Some(segment) = project_segment(rect, view_projection, a, b) {
                    painter.line_segment(segment, stroke);
                }
            }
            if selected && occluder.shape == OccluderShape::Box {
                for axis in 0..2 {
                    let mut local = [0.0, 0.0];
                    local[axis] = occluder.size[axis] / 2.0;
                    let point = matrix.transform_point3(Vec3::new(local[0], local[1], 0.0));
                    if let Some(at) = project_point(rect, view_projection, point) {
                        handles.push((*entity, Grip::Size(axis), at, matrix));
                    }
                }
            }
        }
        for (_, _, at, _) in &handles {
            painter.rect(
                Rect::from_center_size(*at, egui::vec2(HANDLE_SIZE, HANDLE_SIZE)),
                1.0,
                LIGHT_AMBER,
                Stroke::new(1.0, Color32::BLACK),
                egui::StrokeKind::Middle,
            );
        }
        self.drag_light2d_handle(context, response, blocked, &handles, view_projection)
    }

    fn matrix_of(&self, entity: EntityId) -> Mat4 {
        let transform = self.world.world_transform(entity).unwrap_or_default();
        Mat4::from_scale_rotation_translation(
            Vec3::from_array(transform.scale),
            glam::Quat::from_array(transform.rotation),
            Vec3::from_array(transform.position),
        )
    }

    fn drag_light2d_handle(
        &mut self,
        context: &egui::Context,
        response: &Response,
        blocked: bool,
        handles: &[(EntityId, Grip, Pos2, Mat4)],
        view_projection: Mat4,
    ) -> bool {
        let id = egui::Id::new(LIGHT_DRAG);
        let pointer = response.interact_pointer_pos().or(response.hover_pos());
        let pressed = context
            .input(|input| input.pointer.press_origin())
            .or(pointer);
        let near = pressed.and_then(|pointer| {
            handles
                .iter()
                .find(|(_, _, at, _)| at.distance(pointer) <= HANDLE_REACH)
        });
        let mut drag = context.data(|data| data.get_temp::<LightDrag>(id));
        if drag.is_none()
            && !blocked
            && response.drag_started_by(egui::PointerButton::Primary)
            && let Some((entity, grip, _, _)) = near
        {
            drag = Some(LightDrag {
                entity: *entity,
                grip: *grip,
            });
        }
        let Some(active) = drag else {
            return near.is_some() && !blocked;
        };
        let Some((_, _, _, matrix)) = handles
            .iter()
            .find(|(entity, grip, _, _)| *entity == active.entity && *grip == active.grip)
        else {
            context.data_mut(|data| data.remove::<LightDrag>(id));
            return false;
        };
        let depth = matrix.w_axis.z;
        if response.dragged_by(egui::PointerButton::Primary)
            && let Some(pointer) = pointer
            && let Some(world) = unproject_to_plane(response.rect, view_projection, pointer, depth)
        {
            let local = matrix.inverse().transform_point3(world);
            self.write_light2d_size(active, dragged(active.grip, [local.x, local.y]));
        }
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            context.data_mut(|data| data.remove::<LightDrag>(id));
        } else {
            context.data_mut(|data| data.insert_temp(id, active));
        }
        true
    }

    /// Writes the dragged size into the stored payload, keeping every other
    /// field as authored, merged into one undo step for the whole drag.
    fn write_light2d_size(&mut self, drag: LightDrag, size: f32) {
        let (type_name, field, index) = match drag.grip {
            Grip::Radius => (Light2dComponent::TYPE_NAME, "radius", None),
            Grip::Size(axis) => (Light2dOccluderComponent::TYPE_NAME, "size", Some(axis)),
        };
        let Some(mut payload) = self
            .world
            .get(drag.entity)
            .and_then(|data| data.components.get(type_name))
            .cloned()
        else {
            return;
        };
        let Some(object) = payload.as_object_mut() else {
            return;
        };
        match index {
            None => {
                object.insert(field.to_owned(), Value::from(size));
            }
            Some(axis) => {
                let mut pair = object
                    .get(field)
                    .and_then(|value| serde_json::from_value::<[f32; 2]>(value.clone()).ok())
                    .unwrap_or([1.0, 1.0]);
                pair[axis] = size;
                object.insert(field.to_owned(), serde_json::json!(pair));
            }
        }
        let mut buffer = CommandBuffer::new();
        buffer.push(WorldCommand::SetComponent {
            entity: drag.entity,
            type_name: type_name.to_owned(),
            payload,
        });
        let transaction = buffer
            .into_transaction("Resize 2D light")
            .merging(format!("light2d-handle:{}", drag.entity.index()));
        if let Err(error) = self.apply_edit(transaction) {
            self.report(error.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_radius_handle_follows_the_pointer_whichever_way_it_goes() {
        assert!((dragged(Grip::Radius, [3.0, 4.0]) - 5.0).abs() < 1.0e-6);
    }

    #[test]
    fn a_box_handle_grows_the_box_evenly_either_side() {
        assert!((dragged(Grip::Size(0), [-1.5, 9.0]) - 3.0).abs() < 1.0e-6);
        assert!(dragged(Grip::Size(1), [0.0, 0.0]) >= SMALLEST);
    }
}
