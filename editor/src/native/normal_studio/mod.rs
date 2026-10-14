//! The Normal map tab: the picture, its map and the lit surface on a canvas,
//! with the modes along the bottom and the source and map down the left.
//! The settings are properties, so they live in the inspector
//! (`controls.rs`), as a sheet's slice does.

mod assign;
mod canvas;
mod controls;
mod scene;
mod stage;

use std::path::Path;
use std::time::Instant;

use stage::StudioStage;

use eframe::egui::{self, Rect, RichText, Sense, Vec2};

use crate::normal_studio::{Channel, Compare, NormalStudio, StudioMode};
use crate::ui::icons;
use crate::ui::theme::{color, metric, text};
use crate::ui::widgets::{
    button::{self, Intent, Segmented},
    panel,
};

use super::EditorApp;

/// The width of the strip down the left that shows the picture and its map.
const STRIP: f32 = 96.0;

/// What the tab keeps between frames besides the studio itself.
#[derive(Default)]
pub(in crate::native) struct StudioCanvas {
    /// The editor's GPU side of the studio, made the first time it is drawn.
    stage: Option<StudioStage>,
    /// Pictures of the source and the map for the strip, and the map
    /// revision they show.
    thumbs: Option<(u64, egui::TextureHandle, egui::TextureHandle)>,
    /// Whether the split's divider is being dragged.
    dividing: bool,
    /// When the light last moved by itself, to move it on by the time since.
    last_frame: Option<Instant>,
}

impl EditorApp {
    /// Opens `path` in the studio: a picture, or a settings file.
    pub(in crate::native) fn open_normal_studio(&mut self, path: &Path) {
        self.show_nothing();
        self.normal_studio = Some(NormalStudio::open(path));
        self.studio_canvas.thumbs = None;
        self.preferences
            .workspace
            .reveal(crate::dock::Panel::NormalMap);
    }

    /// The Normal map tab.
    pub(in crate::native) fn normal_studio_body(&mut self, ui: &mut egui::Ui) {
        if self.normal_studio.is_none() {
            self.studio_picker(ui);
            return;
        }
        let whole = ui.available_rect_before_wrap();
        let view = self.room_between_overlays(whole, 320.0);
        ui.allocate_rect(whole, Sense::hover());
        ui.painter().rect_filled(whole, 0.0, color::INK);

        let bar = Rect::from_min_max(
            egui::pos2(view.left(), view.bottom() - metric::TOOLBAR_HEIGHT),
            view.right_bottom(),
        );
        let strip = Rect::from_min_max(view.left_top(), egui::pos2(view.left() + STRIP, bar.top()));
        let canvas = Rect::from_min_max(
            egui::pos2(strip.right(), view.top()),
            egui::pos2(view.right(), bar.top()),
        );
        self.advance_studio();
        self.studio_strip(ui, strip);
        self.studio_canvas(ui, canvas);
        self.studio_bar(ui, bar);
    }

    /// Follows the files on disk, makes the map again if a setting moved,
    /// and moves a circling light on.
    fn advance_studio(&mut self) {
        let Some(studio) = self.normal_studio.as_mut() else {
            return;
        };
        studio.follow_disk();
        studio.refresh();
        let now = Instant::now();
        let elapsed = self
            .studio_canvas
            .last_frame
            .map_or(0.0, |last| (now - last).as_secs_f32().min(0.1));
        self.studio_canvas.last_frame = Some(now);
        if studio.view.circling && studio.view.mode.is_lit() {
            studio.light.circle_on(elapsed);
        }
    }

    /// The picture and its map, down the left; a click shows either.
    fn studio_strip(&mut self, ui: &mut egui::Ui, strip: Rect) {
        let Some(studio) = self.normal_studio.as_mut() else {
            return;
        };
        let Some(picture) = studio.picture() else {
            return;
        };
        let stale = self
            .studio_canvas
            .thumbs
            .as_ref()
            .is_none_or(|(revision, ..)| *revision != studio.revision());
        if stale {
            let size = [picture.width as usize, picture.height as usize];
            let source = egui::ColorImage::from_rgba_unmultiplied(size, &picture.rgba);
            let map = egui::ColorImage::from_rgba_unmultiplied(size, studio.map());
            let context = ui.ctx();
            self.studio_canvas.thumbs = Some((
                studio.revision(),
                context.load_texture("studio source", source, egui::TextureOptions::NEAREST),
                context.load_texture("studio map", map, egui::TextureOptions::NEAREST),
            ));
        }
        let Some((_, source, map)) = self.studio_canvas.thumbs.as_ref() else {
            return;
        };
        let painter = ui.painter_at(strip);
        painter.rect_filled(strip, 0.0, color::PANEL);
        let aspect = source.aspect_ratio();
        let side = STRIP - 2.0 * metric::GUTTER;
        let tile = Vec2::new(side, side / aspect.max(0.25)).min(Vec2::splat(side * 1.5));
        let mut top = strip.top() + metric::GUTTER;
        for (texture, label, mode) in [
            (source, "Texture", StudioMode::Texture),
            (map, "Normal map", StudioMode::Normal),
        ] {
            let frame = Rect::from_min_size(egui::pos2(strip.left() + metric::GUTTER, top), tile);
            super::sprite_sheet_view::checkerboard(&painter, frame, 6.0);
            painter.image(
                texture.id(),
                frame,
                Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            let chosen = studio.view.mode == mode;
            painter.rect_stroke(
                frame,
                0.0,
                egui::Stroke::new(
                    if chosen { 2.0 } else { 1.0 },
                    if chosen {
                        color::FORGE_BRIGHT
                    } else {
                        color::LINE
                    },
                ),
                egui::StrokeKind::Outside,
            );
            painter.text(
                egui::pos2(frame.left(), frame.bottom() + 3.0),
                egui::Align2::LEFT_TOP,
                label,
                egui::FontId::proportional(text::NOTE),
                if chosen {
                    color::TEXT
                } else {
                    color::TEXT_MUTED
                },
            );
            let response = ui
                .interact(frame, ui.id().with(label), Sense::click())
                .on_hover_text(format!("Show the {}", label.to_lowercase()));
            if response.clicked() {
                studio.view.mode = mode;
            }
            top = frame.bottom() + 22.0;
        }
    }

    /// The modes, what the mode can show, and Save, along the bottom.
    fn studio_bar(&mut self, ui: &mut egui::Ui, bar: Rect) {
        let mut save = false;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(bar));
        crate::ui::widgets::toolbar::strip(&mut child, bar.height(), |ui| {
            let Some(studio) = self.normal_studio.as_mut() else {
                return;
            };
            let view = &mut studio.view;
            let mut modes = Segmented::new(&mut view.mode);
            for mode in StudioMode::ALL {
                modes = modes.option(mode, mode.label(), mode_tip(mode));
            }
            modes.show(ui);
            crate::ui::widgets::toolbar::divider(ui);
            if view.mode == StudioMode::Normal {
                let mut channels = Segmented::new(&mut view.channel);
                for channel in Channel::ALL {
                    channels =
                        channels.option(channel, channel.label(), "Show this channel as gray");
                }
                channels.show(ui);
            } else if view.mode.is_lit() {
                let mut compare = Segmented::new(&mut view.compare);
                for choice in Compare::ALL {
                    compare = compare.option(choice, choice.label(), compare_tip(choice));
                }
                compare.show(ui);
                ui.checkbox(&mut view.normals_on, "Normal map")
                    .on_hover_text("Off draws the sprite as the game does with no map");
                ui.checkbox(&mut view.circling, "Circle light")
                    .on_hover_text("Move the light round the surface by itself");
            }
            crate::ui::widgets::toolbar::divider(ui);
            let saved = studio.is_saved();
            let label = if saved { "Saved" } else { "Save map" };
            save = ui
                .add_enabled(
                    !saved,
                    egui::Button::new(RichText::new(label).size(text::LABEL)),
                )
                .on_hover_text(format!(
                    "Write {} and its settings beside the picture",
                    file_name(studio.map_path())
                ))
                .clicked();
        });
        if save {
            self.save_normal_map();
        }
        if self
            .normal_studio
            .as_ref()
            .is_some_and(|studio| studio.view.circling)
        {
            ui.ctx().request_repaint();
        }
    }

    /// Writes the map and its settings, and tells the browser.
    pub(in crate::native) fn save_normal_map(&mut self) {
        let Some(studio) = self.normal_studio.as_mut() else {
            return;
        };
        match studio.save() {
            Ok(()) => {
                let name = file_name(studio.map_path());
                self.console.info(format!("Saved normal map {name}"));
                self.refresh_project();
            }
            Err(error) => self.console.error(error),
        }
    }

    /// With nothing open: every image the project holds, one click away.
    fn studio_picker(&mut self, ui: &mut egui::Ui) {
        let images: Vec<(std::path::PathBuf, String)> = self
            .project
            .entries()
            .iter()
            .filter(|entry| entry.kind == crate::project::AssetKind::Texture)
            .filter(|entry| !entry.relative.ends_with("_n.png"))
            .map(|entry| (entry.path.clone(), entry.relative.clone()))
            .collect();
        panel::empty_state(
            ui,
            icons::SPRITE,
            "No picture open",
            "Choose an image to make a normal map from it and see it lit as the game lights it.",
        );
        let mut chosen = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    for (path, relative) in &images {
                        if button::labelled(ui, relative, Intent::Quiet, "Open in the studio")
                            .clicked()
                        {
                            chosen = Some(path.clone());
                        }
                    }
                    if images.is_empty() {
                        panel::note(ui, "This project has no images yet.");
                    }
                });
            });
        if let Some(path) = chosen {
            self.open_normal_studio(&path);
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

const fn mode_tip(mode: StudioMode) -> &'static str {
    match mode {
        StudioMode::Texture => "The picture as it is, unlit",
        StudioMode::Normal => "The map: which way each texel faces",
        StudioMode::Lit => "Lit by the game's renderer. Drag to move the light",
        StudioMode::Inspect => {
            "The lit surface from any angle. Drag to orbit, right-drag to move the light"
        }
    }
}

const fn compare_tip(compare: Compare) -> &'static str {
    match compare {
        Compare::Off => "The whole canvas is the normal-mapped surface",
        Compare::Flat => {
            "Left of the line: lit with no map, as the game draws a sprite without one"
        }
        Compare::Unlit => "Left of the line: the picture with no light at all",
    }
}
