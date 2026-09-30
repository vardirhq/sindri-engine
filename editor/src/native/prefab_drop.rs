//! A prefab dragged from the project browser and dropped in the Scene view.
//!
//! The quickest way to put something where you want it: the drop lands on the
//! grid cell under the pointer when there is one, as a click with the brush
//! armed would, and otherwise where the pointer meets the plane a 2D scene is
//! drawn in.

use std::path::PathBuf;

use eframe::egui::{self, Rect};
use sindri_scene::CameraView;

use crate::prefab::PrefabBrush;

use super::EditorApp;

/// What a prefab row carries while it is being dragged.
#[derive(Clone, Debug)]
pub(crate) struct PrefabDrag(pub(crate) PathBuf);

impl EditorApp {
    /// Places a prefab dropped on the Scene view, and says a drop would.
    pub(super) fn prefab_drop(
        &mut self,
        rect: Rect,
        response: &egui::Response,
        camera: CameraView,
    ) {
        if !self.authoring_enabled() {
            return;
        }
        if response.dnd_hover_payload::<PrefabDrag>().is_some() {
            response.ctx.set_cursor_icon(egui::CursorIcon::Copy);
        }
        let Some(dropped) = response.dnd_release_payload::<PrefabDrag>() else {
            return;
        };
        let Some(pointer) = response.ctx.pointer_latest_pos() else {
            return;
        };
        // Chosen as well as placed, so its panel shows what just arrived.
        self.prefab_brush = Some(PrefabBrush::open(&dropped.0));
        if let Some(cell) = self.cell_under(rect, Some(pointer), camera) {
            self.place_prefab(&cell);
            return;
        }
        let ndc = [
            (pointer.x - rect.min.x) / rect.width().max(1.0) * 2.0 - 1.0,
            1.0 - (pointer.y - rect.min.y) / rect.height().max(1.0) * 2.0,
        ];
        let point = self.view_point(rect.width() / rect.height().max(1.0), ndc);
        self.place_prefab_at(point);
    }
}
