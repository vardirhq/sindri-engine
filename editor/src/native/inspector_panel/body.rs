//! Which of its states the inspector is in, and the strip that says so.
//!
//! The panel is about one thing at a time — an image being sliced, a profile,
//! a block set, a prefab, a file, or an entity — and these two decide which.

use eframe::egui;

use crate::ui::icons;
use crate::ui::theme::color;
use crate::ui::widgets::{panel, toolbar};

use super::super::EditorApp;

impl EditorApp {
    /// Whichever of its states the inspector is in — slicing an image, editing
    /// an entity, previewing a file, or empty.
    pub(in crate::native) fn inspector_body(&mut self, ui: &mut egui::Ui) {
        if self.shown_font.is_none() {
            self.typeface.forget();
        }
        if self.slicer.is_some() {
            self.slicer_panel(ui);
            return;
        }
        if self.profile.is_some() {
            self.profile_panel(ui);
            return;
        }
        if self.block_set.is_some() {
            self.block_set_panel(ui);
            return;
        }
        if self.prefab_brush.is_some() {
            self.prefab_panel(ui);
            return;
        }
        if self.preview.is_some() {
            self.preview_panel(ui);
            return;
        }
        if self.heard.is_some() {
            self.audition_panel(ui);
            return;
        }
        if self.shown_font.is_some() {
            self.typeface_panel(ui);
            return;
        }
        // An empty inspector used to be a blank rectangle, which is
        // indistinguishable from a panel that has stopped working. With nothing
        // in focus, the thing in focus is the scene. The panel used to spend
        // this space on a shrug, and the scene's own name — a real field that
        // round-trips through a save — was shown nowhere at all.
        let Some(entity) = self.selection.primary() else {
            self.scene_section(ui);
            return;
        };
        if self.world.get(entity).is_none() {
            panel::empty_state(
                ui,
                icons::INSPECTOR,
                "That entity is gone",
                "It was removed from the scene while it was selected.",
            );
            return;
        }
        self.inspect_entity(ui, entity);
    }

    /// What the inspector is currently about, said in the strip that names it.
    ///
    /// A chip rather than a second title: the panel is called Inspector however
    /// it is being used, and what changes is its subject.
    pub(in crate::native) fn inspector_actions(&mut self, ui: &mut egui::Ui) {
        let selected = self.selection.len();
        if self.slicer.is_some() {
            toolbar::chip(ui, "Slicing", color::FORGE);
        } else if self.profile.is_some() {
            toolbar::chip(ui, "Profile", color::FORGE);
        } else if self.block_set.is_some() {
            toolbar::chip(ui, "Block set", color::FORGE);
        } else if self.preview.is_some() || self.heard.is_some() || self.shown_font.is_some() {
            toolbar::chip(ui, "Preview", color::TEXT_FAINT);
        } else if selected > 1 {
            // One panel, one subject: a set of fields cannot be about five
            // entities at once. So the panel stays on the last one pointed at
            // and says how many the verbs outside it — Delete, Duplicate, a
            // gizmo drag — would take, rather than letting a selection of five
            // look like a selection of one.
            toolbar::chip(ui, &format!("{selected} selected"), color::FORGE);
        }
    }
}
