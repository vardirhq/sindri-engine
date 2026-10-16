//! The window's frame: brand, mode tabs, project, status and keyboard shortcuts.
use super::{ComposerApp, Mode, scope, theme};
use crate::desktop::{STEPS, project};
use eframe::egui::{self, Key, RichText, Vec2};
use egui_material_icons::icons;

impl ComposerApp {
    pub(super) fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            ui.add_space(4.0);
            let glyph: &str = icons::ICON_GRAPHIC_EQ.into();
            ui.label(RichText::new(glyph).size(20.0).color(theme::FORGE));
            ui.label(
                RichText::new("SINDRI")
                    .strong()
                    .size(14.0)
                    .color(theme::TEXT),
            );
            ui.label(RichText::new("SOUND LAB").size(14.0).color(theme::FORGE));
            ui.add_space(18.0);
            for (mode, icon, name) in [
                (Mode::Music, icons::ICON_PIANO, "Music"),
                (Mode::Sfx, icons::ICON_BOLT, "Sound FX"),
            ] {
                let text = RichText::new(theme::label(icon, name)).size(13.5);
                if theme::segment(ui, self.mode == mode, text) {
                    self.mode = mode;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                let (name, color) = self.project.as_ref().map_or_else(
                    || ("Choose project…".to_owned(), theme::MUTED),
                    |root| (project::name(root), theme::TEXT),
                );
                let chip = egui::Button::new(
                    RichText::new(theme::label(icons::ICON_FOLDER, &name)).color(color),
                )
                .fill(theme::WELL)
                .stroke(egui::Stroke::new(1.0, theme::LINE));
                if ui
                    .add(chip)
                    .on_hover_text("The Sindri project Send writes sounds into")
                    .clicked()
                {
                    self.pick_project();
                }
                ui.label(RichText::new("PROJECT").size(10.5).color(theme::FAINT));
                ui.add_space(12.0);
                if let Some(samples) = self.scope_samples() {
                    scope::meter(ui, &samples, Vec2::new(8.0, 26.0));
                    scope::oscilloscope(ui, &samples, Vec2::new(150.0, 30.0), theme::FORGE);
                }
            });
        });
    }

    pub(super) fn scope_samples(&self) -> Option<Vec<f32>> {
        let transport = self.transport.as_ref()?;
        let scope = transport.scope.lock().ok()?;
        Some(scope.clone())
    }

    pub(super) fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            let failed = [
                "fail",
                "could not",
                "unavailable",
                "cannot",
                "must",
                "no sindri",
            ]
            .iter()
            .any(|word| self.status.to_lowercase().contains(word));
            let (icon, color) = if failed {
                (icons::ICON_ERROR, theme::DANGER)
            } else {
                (icons::ICON_CHECK_CIRCLE, theme::FAINT)
            };
            ui.label(
                RichText::new(theme::label(icon, &self.status))
                    .size(11.5)
                    .color(color),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let keys = match self.mode {
                    Mode::Music => {
                        "Space play · Ctrl+Z/Y undo · Del delete · arrows nudge · Ctrl+S save"
                    }
                    Mode::Sfx => "Space play · M mutate · R new variation · 1–8 pick a kind",
                };
                ui.label(RichText::new(keys).size(11.0).color(theme::FAINT));
            });
        });
    }

    /// Keys act only while no text field has focus.
    pub(super) fn shortcuts(&mut self, ui: &egui::Ui) {
        let ctx = ui.ctx().clone();
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (keys, command, shift) = ctx.input(|i| {
            let keys: Vec<Key> = [
                Key::Space,
                Key::Z,
                Key::Y,
                Key::S,
                Key::M,
                Key::R,
                Key::Delete,
                Key::Backspace,
                Key::ArrowUp,
                Key::ArrowDown,
                Key::ArrowLeft,
                Key::ArrowRight,
                Key::Num1,
                Key::Num2,
                Key::Num3,
                Key::Num4,
                Key::Num5,
                Key::Num6,
                Key::Num7,
                Key::Num8,
            ]
            .into_iter()
            .filter(|key| i.key_pressed(*key))
            .collect();
            (keys, i.modifiers.command, i.modifiers.shift)
        });
        for key in keys {
            match (self.mode, key) {
                (Mode::Music, Key::Space) => self.toggle_play(),
                (Mode::Music, Key::Z) if command && shift => self.redo_edit(),
                (Mode::Music, Key::Z) if command => self.undo_edit(),
                (Mode::Music, Key::Y) if command => self.redo_edit(),
                (Mode::Music, Key::S) if command => self.save_song(shift),
                (Mode::Music, Key::Delete | Key::Backspace) => self.delete_selected(),
                (Mode::Music, Key::ArrowUp) => self.nudge_selected(if shift { 12 } else { 1 }, 0),
                (Mode::Music, Key::ArrowDown) => {
                    self.nudge_selected(if shift { -12 } else { -1 }, 0);
                }
                (Mode::Music, Key::ArrowLeft) => self.nudge_selected(0, -1),
                (Mode::Music, Key::ArrowRight) => self.nudge_selected(0, 1),
                (Mode::Sfx, Key::Space) => self.audition(),
                (Mode::Sfx, Key::M) => {
                    self.sfx.mutate();
                    self.audition();
                }
                (Mode::Sfx, Key::R) => {
                    self.sfx.generate(self.sfx.file.kind);
                    self.audition();
                }
                (Mode::Sfx, number) => {
                    let index = [
                        Key::Num1,
                        Key::Num2,
                        Key::Num3,
                        Key::Num4,
                        Key::Num5,
                        Key::Num6,
                        Key::Num7,
                        Key::Num8,
                    ]
                    .iter()
                    .position(|k| *k == number);
                    if let Some(index) = index {
                        self.sfx.generate(crate::desktop::sfx::Kind::ALL[index]);
                        self.audition();
                    }
                }
                _ => {}
            }
        }
    }

    fn delete_selected(&mut self) {
        if let Some(index) = self.selected_note {
            self.save_undo();
            self.scores[self.selected][self.edit_track].remove(index);
            self.selected_note = None;
            self.finish_edit();
        }
    }

    fn nudge_selected(&mut self, pitch: i32, step: i32) {
        let Some(index) = self.selected_note else {
            return;
        };
        let Some(mut note) = self.scores[self.selected][self.edit_track]
            .get(index)
            .copied()
        else {
            return;
        };
        note.pitch = (note.pitch + pitch).clamp(0, 127);
        let start = i32::try_from(note.start).expect("step") + step;
        let last = i32::try_from(STEPS - note.len).expect("step");
        note.start = usize::try_from(start.clamp(0, last)).expect("clamped");
        self.save_undo();
        self.scores[self.selected][self.edit_track][index] = note;
        self.finish_edit();
    }
}
