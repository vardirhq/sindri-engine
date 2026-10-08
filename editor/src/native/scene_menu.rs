//! The Scene view's right-click menu.
//!
//! The right button's drag orbits the camera, and egui tells a click from a
//! drag, so a click that does not move opens this instead and the two never
//! meet (`docs/editor-authoring-audit.md` §6). What it offers is about the
//! view and the point clicked: framing, and making or pasting something
//! there. Where "there" is, is decided as a dropped prefab's is: where the
//! pointer meets the plane a 2D scene is drawn in.

use eframe::egui::{self, Rect, Response};
use glam::Vec3;
use sindri_core::Transform3D;

use crate::space::{EntitySpace, space_of};
use crate::ui::widgets::menu;

use super::EditorApp;
use super::camera::pan_to_centre;

const SCENE_MENU_POINT: &str = "sindri-scene-menu-point";

/// What the menu was asked for.
#[derive(Clone, Copy)]
enum SceneAsk {
    FrameSelected,
    FrameAll,
    CreateHere([f32; 2]),
    PasteHere([f32; 2]),
}

impl EditorApp {
    /// Opens the menu on a right-click in the Scene view and does what it
    /// asks.
    pub(super) fn scene_menu(&mut self, rect: Rect, response: &Response) {
        let id = egui::Id::new(SCENE_MENU_POINT);
        // Where the click landed, kept while the menu is open: the pointer
        // moves on to the entries.
        if response.secondary_clicked()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let ndc = [
                (pointer.x - rect.min.x) / rect.width().max(1.0) * 2.0 - 1.0,
                1.0 - (pointer.y - rect.min.y) / rect.height().max(1.0) * 2.0,
            ];
            let point = self.view_point(rect.width() / rect.height().max(1.0), ndc);
            response.ctx.data_mut(|data| data.insert_temp(id, point));
        }
        let point = response
            .ctx
            .data(|data| data.get_temp::<[f32; 2]>(id))
            .unwrap_or_default();
        let editable = self.world_editable();
        let selected = !self.selection.is_empty();
        let can_paste = self.can_paste();
        let mut asked = None;
        menu::on_right_click(response, |ui| {
            menu::subject(ui, "Scene view");
            if ui
                .add_enabled(selected, menu::entry("Frame selected", "F"))
                .clicked()
            {
                asked = Some(SceneAsk::FrameSelected);
                ui.close();
            }
            if menu::item(ui, "Frame everything").clicked() {
                asked = Some(SceneAsk::FrameAll);
                ui.close();
            }
            ui.separator();
            ui.add_enabled_ui(editable, |ui| {
                if menu::item(ui, "Create Empty here").clicked() {
                    asked = Some(SceneAsk::CreateHere(point));
                    ui.close();
                }
                if ui
                    .add_enabled(can_paste, menu::entry("Paste here", ""))
                    .clicked()
                {
                    asked = Some(SceneAsk::PasteHere(point));
                    ui.close();
                }
            });
        });
        match asked {
            None => {}
            Some(SceneAsk::FrameSelected) => self.focus_selection(),
            Some(SceneAsk::FrameAll) => self.frame_everything(),
            Some(SceneAsk::CreateHere([x, y])) => self.create_entity_placed(
                None,
                Transform3D {
                    position: [x, y, 0.0],
                    ..Transform3D::default()
                },
            ),
            Some(SceneAsk::PasteHere([x, y])) => self.paste(None, Some(Vec3::new(x, y, 0.0))),
        }
    }

    /// Centres the Scene view on the middle of everything in the world, as
    /// Frame selected centres it on the selection.
    fn frame_everything(&mut self) {
        let placed: Vec<Vec3> = self
            .world
            .entities()
            .filter(|(entity, _)| space_of(&self.world, *entity) == EntitySpace::World)
            .filter_map(|(entity, _)| self.world.world_transform(entity))
            .map(|transform| Vec3::from_array(transform.position))
            .collect();
        if placed.is_empty() {
            return;
        }
        let (low, high) = placed.iter().fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(low, high), point| (low.min(*point), high.max(*point)),
        );
        let Ok(Some(camera)) = self.scene.world_camera(&self.world, self.scene_camera()) else {
            return;
        };
        self.viewport_pan = pan_to_centre(camera, self.viewport_pan, (low + high) / 2.0);
    }
}
