//! The Sprite sheet tab: an image filling the canvas with the slice drawn
//! over it; and the inspector's controls that cut it.
//!
//! Slicing used to happen inside the inspector, where a sheet was shown at
//! the inspector's width — a few hundred pixels for a texture of two
//! thousand, or a 16-pixel sprite blown up past readability. Whether the
//! cells fall on the frames is the whole job, and that needs the room a
//! top-row tab has: the picture zooms with the wheel, pans with the middle or
//! right button, and stays pixel-sharp at any magnification.

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use crate::slicer::Slicer;
use crate::ui::icons;
use crate::ui::theme::{color, metric, radius, text};
use crate::ui::widgets::{
    button::{self, Intent},
    panel,
};

use super::EditorApp;
use super::slicer_view::{slice_grid, slice_names};

/// How far in and out the picture goes.
const ZOOM: std::ops::RangeInclusive<f32> = 0.05..=64.0;

/// Where the picture sits in the canvas: its scale and where its centre is,
/// relative to the canvas's centre. Reset whenever another image is opened.
#[derive(Clone, Debug, Default)]
pub(super) struct SheetCamera {
    /// Screen pixels per image pixel; `None` until fitted to the canvas.
    zoom: Option<f32>,
    offset: Vec2,
    /// The image this was fitted to, so a new one is fitted afresh.
    fitted_for: Option<std::path::PathBuf>,
}

impl SheetCamera {
    /// The largest whole or fractional scale that shows the whole image with
    /// a margin, never above eight times for a tiny sheet.
    fn fit(canvas: Rect, image: Vec2) -> f32 {
        let room = canvas.size() - Vec2::splat(48.0);
        (room.x / image.x)
            .min(room.y / image.y)
            .clamp(*ZOOM.start(), 8.0)
    }
}

/// An image dimension as a length to lay out with.
#[allow(clippy::cast_precision_loss)]
fn pixels(value: u32) -> f32 {
    value as f32
}

impl EditorApp {
    /// The Sprite sheet tab.
    pub(super) fn sprite_sheet_body(&mut self, ui: &mut egui::Ui) {
        if self.slicer.is_none() {
            self.sheet_picker(ui);
            return;
        }
        // The picture runs under the overlays, as a scene does, so it can be
        // panned anywhere; a fit fills the gap between them. The slice's
        // settings are properties, and live in the inspector.
        let whole = ui.available_rect_before_wrap();
        let view = self.room_between_overlays(whole, 320.0);
        ui.allocate_rect(whole, Sense::hover());
        if let Some(slicer) = self.slicer.as_mut() {
            sheet_canvas(ui, whole, view, slicer, &mut self.sheet_camera);
        }
    }

    /// The inspector while an image is open: its card, the slice, the names
    /// and Save.
    pub(super) fn slice_inspector(&mut self, ui: &mut egui::Ui) {
        let Some(slicer) = self.slicer.as_mut() else {
            return;
        };
        let mut save = false;
        let mut close = false;
        egui::ScrollArea::vertical()
            .id_salt("slice inspector")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                (save, close) = sheet_controls(ui, slicer);
            });
        if save {
            self.save_slice();
        }
        if close {
            self.slicer = None;
        }
    }

    /// Writes the slice, and tells the project browser its sprites changed.
    fn save_slice(&mut self) {
        let Some(slicer) = self.slicer.as_mut() else {
            return;
        };
        let name = slicer.name();
        if slicer.save() {
            self.console.info(format!("Sliced {name}"));
            // The browser lists a texture's sprites from the sidecar, so a
            // save it did not notice would leave the new names invisible.
            self.refresh_project();
        } else if let Some(problem) = slicer.problem.clone() {
            self.console.error(problem);
        }
    }

    /// With nothing open: every image the project holds, one click from
    /// being sliced.
    fn sheet_picker(&mut self, ui: &mut egui::Ui) {
        let images: Vec<(std::path::PathBuf, String)> = self
            .project
            .entries()
            .iter()
            .filter(|entry| entry.kind == crate::project::AssetKind::Texture)
            .map(|entry| (entry.path.clone(), entry.relative.clone()))
            .collect();
        panel::empty_state(
            ui,
            icons::SPRITE,
            "No image open",
            "Choose an image here or in the project browser to cut it into named sprites.",
        );
        let mut chosen = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    for (path, relative) in &images {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(relative).size(text::LABEL).color(color::TEXT),
                                )
                                .min_size(Vec2::new(280.0, metric::CONTROL_HEIGHT)),
                            )
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
            self.select_asset(&path);
        }
    }
}

/// The image, zoomed and panned, with every cell outlined, the picked one
/// bright, and named cells labelled once there is room to read them.
/// `canvas` is all the picture may be drawn and panned over; `view` is the
/// part of it nothing covers, which a fit fills and the readout sits in.
fn sheet_canvas(
    ui: &mut egui::Ui,
    canvas: Rect,
    view: Rect,
    slicer: &mut Slicer,
    camera: &mut SheetCamera,
) {
    let response = ui.interact(
        canvas,
        ui.id().with("sprite sheet canvas"),
        Sense::click_and_drag(),
    );
    let painter = ui.painter_at(canvas);
    painter.rect_filled(canvas, 0.0, color::INK);
    let (width, height) = slicer.size();
    let rects = slicer.cell_rects();
    let selected = slicer.selected;
    let named: Vec<(u32, String)> = slicer
        .named()
        .into_iter()
        .map(|(index, name)| (index, name.to_owned()))
        .collect();
    let Some(texture) = slicer.texture(ui.ctx()).map(egui::TextureHandle::id) else {
        painter.text(
            canvas.center(),
            egui::Align2::CENTER_CENTER,
            "No preview: this build cannot read the image",
            egui::FontId::proportional(text::LABEL),
            color::TEXT_MUTED,
        );
        return;
    };
    if width == 0 || height == 0 {
        return;
    }
    let image = Vec2::new(pixels(width), pixels(height));
    if camera.fitted_for.as_deref() != Some(slicer.path()) || camera.zoom.is_none() {
        camera.zoom = Some(SheetCamera::fit(view, image));
        camera.offset = view.center() - canvas.center();
        camera.fitted_for = Some(slicer.path().to_owned());
    }
    let zoom = steer(ui, &response, canvas, camera);

    let shown = Rect::from_center_size(canvas.center() + camera.offset, image * zoom);
    checkerboard(&painter, shown.intersect(canvas), 8.0);
    painter.image(
        texture,
        shown,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    painter.rect_stroke(
        shown,
        0.0,
        Stroke::new(1.0, color::LINE),
        egui::StrokeKind::Outside,
    );

    let cell_rect = |[x, y, w, h]: [f32; 4]| {
        Rect::from_min_size(
            Pos2::new(
                shown.left() + x * shown.width(),
                shown.top() + y * shown.height(),
            ),
            Vec2::new(w * shown.width(), h * shown.height()),
        )
    };
    let drawn: Vec<Rect> = rects.iter().map(|cell| cell_rect(*cell)).collect();
    draw_cells(&painter, &drawn, selected, &named);

    // Picked by hit-testing the drawn rects, so a click lands on the cell it
    // looks like it landed on even when gutters mean the cells do not tile.
    if response.clicked_by(egui::PointerButton::Primary)
        && let Some(pointer) = response.interact_pointer_pos()
        && let Some(picked) = drawn
            .iter()
            .position(|cell| cell.contains(pointer))
            .and_then(|index| u32::try_from(index).ok())
    {
        slicer.selected = picked;
    }

    readout(ui, view, camera, zoom, (width, height));
}

/// The wheel zooms about the pointer, so what is under it stays under it;
/// the middle or right button pans. Answers the zoom to draw at.
fn steer(ui: &egui::Ui, response: &egui::Response, canvas: Rect, camera: &mut SheetCamera) -> f32 {
    let mut zoom = camera.zoom.unwrap_or(1.0);

    // The wheel zooms about the pointer, so what is under it stays under it.
    if let Some(pointer) = response.hover_pos() {
        let scroll = ui.input(|input| input.smooth_scroll_delta.y);
        if scroll != 0.0 {
            let before = zoom;
            zoom = (zoom * (scroll * 0.0025).exp()).clamp(*ZOOM.start(), *ZOOM.end());
            let from_centre = pointer - canvas.center() - camera.offset;
            camera.offset -= from_centre * (zoom / before - 1.0);
        }
    }
    let panning = response.dragged_by(egui::PointerButton::Middle)
        || response.dragged_by(egui::PointerButton::Secondary);
    if panning {
        camera.offset += response.drag_delta();
    }
    camera.zoom = Some(zoom);
    zoom
}

/// Every cell outlined, the picked one bright, and named cells labelled
/// once there is room to read them.
fn draw_cells(painter: &egui::Painter, cells: &[Rect], selected: u32, named: &[(u32, String)]) {
    let faint = Stroke::new(1.0, color::FORGE.gamma_multiply(0.45));
    for (index, cell) in cells.iter().enumerate() {
        if u32::try_from(index).is_ok_and(|index| index == selected) {
            continue;
        }
        painter.rect_stroke(*cell, 0.0, faint, egui::StrokeKind::Inside);
    }
    for (index, name) in named {
        let Some(cell) = cells.get(*index as usize) else {
            continue;
        };
        if cell.width() < 44.0 {
            continue;
        }
        painter.with_clip_rect(*cell).text(
            cell.left_top() + Vec2::new(3.0, 2.0),
            egui::Align2::LEFT_TOP,
            name,
            egui::FontId::proportional(text::NOTE),
            color::FORGE_BRIGHT,
        );
    }
    if let Some(cell) = cells.get(selected as usize) {
        painter.rect_stroke(
            *cell,
            0.0,
            Stroke::new(2.0, color::FORGE_BRIGHT),
            egui::StrokeKind::Inside,
        );
    }
}

/// What the view is at, and the way back to all of it.
fn readout(
    ui: &mut egui::Ui,
    canvas: Rect,
    camera: &mut SheetCamera,
    zoom: f32,
    (width, height): (u32, u32),
) {
    let readout = format!("{:.0}%  ·  {width} × {height} px", zoom * 100.0);
    ui.painter_at(canvas).text(
        canvas.left_bottom() + Vec2::new(10.0, -10.0),
        egui::Align2::LEFT_BOTTOM,
        readout,
        egui::FontId::proportional(text::NOTE),
        color::TEXT_MUTED,
    );
    let fit = Rect::from_min_size(
        canvas.right_bottom() - Vec2::new(64.0, 34.0),
        Vec2::new(56.0, 26.0),
    );
    if ui
        .put(
            fit,
            egui::Button::new(RichText::new("Fit").size(text::LABEL)),
        )
        .on_hover_text("Show the whole image (wheel to zoom, middle or right drag to pan)")
        .clicked()
    {
        camera.zoom = None;
    }
}

/// The squares behind a transparent sheet, so transparency reads as such.
fn checkerboard(painter: &egui::Painter, area: Rect, size: f32) {
    if area.width() <= 0.0 || area.height() <= 0.0 {
        return;
    }
    painter.rect_filled(area, 0.0, Color32::from_gray(38));
    let dark = Color32::from_gray(30);
    let mut y = area.top();
    let mut row = 0_u32;
    while y < area.bottom() {
        let mut x = area.left() + if row.is_multiple_of(2) { 0.0 } else { size };
        while x < area.right() {
            let square = Rect::from_min_size(Pos2::new(x, y), Vec2::splat(size)).intersect(area);
            painter.rect_filled(square, 0.0, dark);
            x += size * 2.0;
        }
        y += size;
        row += 1;
    }
}

/// The image's card, the slice, the names and Save. Answers whether Save or
/// close was pressed.
fn sheet_controls(ui: &mut egui::Ui, slicer: &mut Slicer) -> (bool, bool) {
    let mut save = false;
    let mut close = false;
    let (width, height) = slicer.size();
    egui::Frame::new()
        .fill(color::RAISED)
        .stroke(Stroke::new(1.0, color::LINE_SOFT))
        .corner_radius(radius())
        .inner_margin(egui::Margin::symmetric(8, 6))
        .outer_margin(egui::Margin::symmetric(metric::GUTTER_EDGE, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    icons::SPRITE
                        .outlined()
                        .rich_text()
                        .size(17.0)
                        .color(color::FORGE),
                );
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(slicer.name())
                            .size(text::BODY)
                            .color(color::TEXT),
                    );
                    ui.label(
                        RichText::new(format!("{width} × {height} px"))
                            .size(text::NOTE)
                            .color(color::TEXT_FAINT),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button::row_icon(ui, icons::CLOSE, Intent::Quiet, "Close this image")
                        .clicked()
                    {
                        close = true;
                    }
                });
            });
        });
    slice_grid(ui, slicer);
    ui.add_space(6.0);
    slice_names(ui, slicer);
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(metric::GUTTER);
        if button::labelled(
            ui,
            "Save slice",
            Intent::Primary,
            "Write the sheet beside the image so the project can use its sprites",
        )
        .clicked()
        {
            save = true;
        }
    });
    if let Some(problem) = &slicer.problem {
        panel::problem(ui, problem);
    }
    ui.add_space(8.0);
    (save, close)
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Rect, Vec2, pos2};

    use super::SheetCamera;

    #[test]
    fn a_sheet_fits_the_canvas_and_a_tiny_one_is_not_blown_up_past_eight() {
        let canvas = Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(848.0, 448.0));
        let big = SheetCamera::fit(canvas, Vec2::new(2000.0, 1000.0));
        assert!((big - 0.4).abs() < 1e-4, "{big}");
        let tiny = SheetCamera::fit(canvas, Vec2::new(16.0, 16.0));
        assert!((tiny - 8.0).abs() < f32::EPSILON);
    }
}
