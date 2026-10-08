//! Joints, 3D colliders, characters' footing and effects' reach as the Scene
//! view draws them.
//!
//! [`sindri_scene::shape_gizmos`] works out where each is from what the
//! simulation is given; this draws them. Joints and 3D colliders are drawn
//! always, faint until selected, because a joint lives on an entity of its
//! own that nothing else in the view shows, and a 3D collider is as invisible
//! as a 2D one. A character's footing and an effect's reach are drawn for the
//! selection only: they are about tuning one thing, and a scene full of them
//! would bury what is being tuned. A click on a joint's line selects it.

use eframe::egui::{self, Color32, LayerId, Order, Painter, Pos2, Rect, Response, Stroke};
use glam::{Mat4, Vec3};
use sindri_core::EntityId;
use sindri_scene::{GizmoKind, ShapeGizmo, shape_gizmos};

use super::EditorApp;
use super::projection::{distance_to_segment, project_point, project_segment};

const SHAPE_LAYER: &str = "sindri-shape-gizmos";
const SHAPE_PICK_STATE: &str = "sindri-shape-pick";

/// How near a click has to land on a joint's line to pick it.
const PICK_DISTANCE: f32 = 6.0;
const MARK_RADIUS: f32 = 3.0;

const JOINT_BLUE: Color32 = Color32::from_rgb(110, 190, 250);
/// The 2D collider overlay's green, so a 3D collider reads as the same thing.
const COLLIDER_GREEN: Color32 = Color32::from_rgb(140, 250, 140);
const FOOTING_ORANGE: Color32 = Color32::from_rgb(250, 160, 90);
const REACH_PINK: Color32 = Color32::from_rgb(240, 130, 220);

fn tone(kind: GizmoKind) -> Color32 {
    match kind {
        GizmoKind::Joint => JOINT_BLUE,
        GizmoKind::Collider3d => COLLIDER_GREEN,
        GizmoKind::Character => FOOTING_ORANGE,
        GizmoKind::EffectReach => REACH_PINK,
    }
}

/// Whether a gizmo is drawn at all: footing and reach only when selected.
fn shown(kind: GizmoKind, selected: bool) -> bool {
    selected || matches!(kind, GizmoKind::Joint | GizmoKind::Collider3d)
}

/// One gizmo on screen.
struct Drawn {
    entity: EntityId,
    kind: GizmoKind,
    segments: Vec<[Pos2; 2]>,
    marks: Vec<Pos2>,
}

fn project(gizmo: &ShapeGizmo, rect: Rect, view_projection: Mat4) -> Drawn {
    Drawn {
        entity: gizmo.entity,
        kind: gizmo.kind,
        segments: gizmo
            .strokes
            .iter()
            .flat_map(sindri_scene::GizmoStroke::segments)
            .filter_map(|(a, b)| {
                project_segment(rect, view_projection, Vec3::from(a), Vec3::from(b))
            })
            .collect(),
        marks: gizmo
            .marks
            .iter()
            .filter_map(|&mark| project_point(rect, view_projection, Vec3::from(mark)))
            .collect(),
    }
}

fn paint(painter: &Painter, drawn: &Drawn, selected: bool) {
    let color = if selected {
        tone(drawn.kind)
    } else {
        tone(drawn.kind).gamma_multiply(0.5)
    };
    let stroke = Stroke::new(if selected { 1.75 } else { 1.0 }, color);
    for segment in &drawn.segments {
        painter.line_segment(*segment, stroke);
    }
    for mark in &drawn.marks {
        painter.circle(*mark, MARK_RADIUS, color, Stroke::new(1.0, Color32::BLACK));
    }
}

impl EditorApp {
    /// Draws the shape gizmos, and lets a click on a joint select it.
    ///
    /// The click is carried to the next frame, as a light's is, so it lands
    /// after the world's own picking and wins over whatever is behind it.
    pub(super) fn shape_gizmo_overlay(
        &mut self,
        context: &egui::Context,
        response: &Response,
        tool_owns_primary: bool,
    ) {
        let pick_state = egui::Id::new(SHAPE_PICK_STATE);
        if let Some(Some(entity)) =
            context.data_mut(|data| data.remove_temp::<Option<EntityId>>(pick_state))
        {
            self.select(Some(entity));
        }
        let rect = response.rect;
        let aspect = rect.width() / rect.height().max(1.0);
        let Some(camera) = self
            .scene
            .world_camera_for_viewport(&self.world, aspect, self.scene_camera())
            .ok()
            .flatten()
        else {
            return;
        };
        let drawn: Vec<(Drawn, bool)> = shape_gizmos(&self.world, self.scene.components())
            .iter()
            .filter_map(|gizmo| {
                let selected = self.selection.contains(gizmo.entity);
                shown(gizmo.kind, selected)
                    .then(|| (project(gizmo, rect, camera.view_projection), selected))
            })
            .collect();
        if !tool_owns_primary
            && response.clicked_by(egui::PointerButton::Primary)
            && let Some(pointer) = response.interact_pointer_pos()
            && let Some((joint, _)) = drawn.iter().rev().find(|(drawn, _)| {
                drawn.kind == GizmoKind::Joint
                    && drawn
                        .segments
                        .iter()
                        .any(|&[a, b]| distance_to_segment(pointer, a, b) <= PICK_DISTANCE)
            })
        {
            let entity = joint.entity;
            context.data_mut(|data| data.insert_temp(pick_state, Some(entity)));
        }
        let painter = context
            // Background, so the floating panels stay over it; a layer of
            // its own, which egui paints after the Scene view's.
            .layer_painter(LayerId::new(Order::Background, egui::Id::new(SHAPE_LAYER)))
            .with_clip_rect(rect);
        for (drawn, selected) in &drawn {
            paint(&painter, drawn, *selected);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footing_and_reach_wait_for_the_selection() {
        assert!(shown(GizmoKind::Joint, false));
        assert!(shown(GizmoKind::Collider3d, false));
        assert!(!shown(GizmoKind::Character, false));
        assert!(!shown(GizmoKind::EffectReach, false));
        assert!(shown(GizmoKind::EffectReach, true));
    }
}
