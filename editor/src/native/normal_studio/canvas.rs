//! The studio's canvas: the surface drawn by the game's renderer, the split
//! that compares it, the light standing over it, and the gestures that move
//! the light and the camera.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use glam::Vec3;
use sindri_scene::ViewCamera;

use super::super::frame::physical_viewport_dimension;
use super::super::projection::project_point;
use super::StudioCanvas;
use super::scene::{Look, camera_for, onto_plane};
use super::stage::{Pane, StudioStage};
use crate::normal_studio::{Compare, NormalStudio, StudioMode};
use crate::ui::theme::{color, text};

use super::super::EditorApp;

/// How near the divider, in points, a drag has to start to move it.
const DIVIDER_REACH: f32 = 8.0;

impl EditorApp {
    /// Draws the surface into `canvas` and answers the gestures over it.
    pub(super) fn studio_canvas(&mut self, ui: &mut egui::Ui, canvas: Rect) {
        let response = ui.interact(
            canvas,
            ui.id().with("normal studio canvas"),
            Sense::click_and_drag(),
        );
        let Some(studio) = self.normal_studio.as_mut() else {
            return;
        };
        let painter = ui.painter_at(canvas);
        if studio.picture().is_none() || canvas.width() < 8.0 || canvas.height() < 8.0 {
            let why = studio
                .problem
                .clone()
                .unwrap_or_else(|| "Nothing to show".to_owned());
            painter.text(
                canvas.center(),
                egui::Align2::CENTER_CENTER,
                why,
                egui::FontId::proportional(text::LABEL),
                color::TEXT_MUTED,
            );
            return;
        }
        let kept = &mut self.studio_canvas;
        let stage = kept
            .stage
            .get_or_insert_with(|| StudioStage::new(&self.render_state));
        let aspect = canvas.width() / canvas.height();
        let seen = stage.view_camera(aspect, camera_for(studio, aspect));
        steer(ui, &response, canvas, studio, kept, seen.as_ref());
        let Some(stage) = kept.stage.as_mut() else {
            return;
        };
        if let Err(error) = stage.sync(&self.render_state, studio) {
            self.console.fail(&error, None);
            return;
        }
        let scale = ui.ctx().pixels_per_point();
        let size = (
            physical_viewport_dimension(canvas.width(), scale),
            physical_viewport_dimension(canvas.height(), scale),
        );
        let camera = camera_for(studio, aspect);
        let (main, other) = looks(studio);
        let whole = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        let drawn = stage.draw(&mut self.renderers, Pane::Main, main, size, camera);
        let compared =
            other.map(|look| stage.draw(&mut self.renderers, Pane::Other, look, size, camera));
        let main = match drawn {
            Ok(texture) => texture,
            Err(error) => {
                self.console.fail(&error, None);
                return;
            }
        };
        painter.image(main, canvas, whole, Color32::WHITE);
        if let Some(Ok(other)) = compared {
            let divider = canvas.left() + studio.view.split * canvas.width();
            let left = Rect::from_min_max(canvas.min, Pos2::new(divider, canvas.bottom()));
            ui.painter_at(left)
                .image(other, canvas, whole, Color32::WHITE);
            paint_divider(&painter, canvas, divider, studio.view.compare);
        }
        let seen = stage.view_camera(aspect, camera);
        if studio.view.mode.is_lit()
            && let Some(seen) = seen
        {
            paint_light(&painter, canvas, &seen, studio);
        }
        readout(&painter, canvas, studio);
    }
}

/// What the canvas shows, and what the other side of a split shows.
fn looks(studio: &NormalStudio) -> (Look, Option<Look>) {
    let view = &studio.view;
    match view.mode {
        StudioMode::Texture => (Look::Unlit, None),
        StudioMode::Normal => (Look::Channel, None),
        StudioMode::Lit | StudioMode::Inspect => {
            let main = if view.normals_on {
                Look::Mapped
            } else {
                Look::Flat
            };
            let other = match view.compare {
                Compare::Off => None,
                Compare::Flat => Some(Look::Flat),
                Compare::Unlit => Some(Look::Unlit),
            };
            (main, other)
        }
    }
}

/// The wheel zooms; in the lit view a drag moves the light, and in 3D a drag
/// orbits and a right-drag moves the light; elsewhere the middle or right
/// button pans; a drag that starts on the split's divider moves it; a
/// double-click puts the camera back.
fn steer(
    ui: &egui::Ui,
    response: &egui::Response,
    canvas: Rect,
    studio: &mut NormalStudio,
    state: &mut StudioCanvas,
    seen: Option<&ViewCamera>,
) {
    let view = &mut studio.view;
    let inspecting = view.mode == StudioMode::Inspect;
    if response.hovered() {
        let scroll = ui.input(|input| input.smooth_scroll_delta.y) / 40.0;
        if scroll != 0.0 {
            if inspecting {
                view.orbit.zoom_by(scroll);
            } else {
                view.zoom_by(scroll);
            }
        }
    }
    if response.double_clicked() {
        view.reset_camera();
        return;
    }
    let splitting = view.mode.is_lit() && view.compare != Compare::Off;
    let divider = canvas.left() + view.split * canvas.width();
    if response.drag_started_by(egui::PointerButton::Primary) {
        state.dividing = splitting
            && response
                .interact_pointer_pos()
                .is_some_and(|pointer| (pointer.x - divider).abs() <= DIVIDER_REACH);
    }
    if response.drag_stopped() {
        state.dividing = false;
    }
    let pointer = response.interact_pointer_pos();
    if state.dividing {
        if let Some(pointer) = pointer {
            view.split = ((pointer.x - canvas.left()) / canvas.width()).clamp(0.02, 0.98);
        }
        return;
    }
    let primary = response.dragged_by(egui::PointerButton::Primary)
        || response.clicked_by(egui::PointerButton::Primary);
    let secondary = response.dragged_by(egui::PointerButton::Secondary)
        || response.clicked_by(egui::PointerButton::Secondary);
    let places_light = if inspecting {
        secondary
    } else {
        view.mode == StudioMode::Lit && primary
    };
    if places_light
        && let (Some(pointer), Some(seen)) = (pointer, seen)
        && let Some(point) = onto_plane(seen, canvas, pointer)
    {
        view.circling = false;
        studio.light.place(point);
        return;
    }
    if inspecting && response.dragged_by(egui::PointerButton::Primary) {
        let delta = response.drag_delta();
        view.orbit.turn([delta.x, delta.y]);
    } else if !inspecting
        && (response.dragged_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Secondary))
    {
        let delta = response.drag_delta() / (canvas.height() * 0.5);
        view.pan[0] += delta.x;
        view.pan[1] -= delta.y;
    }
}

/// The line the split falls on, with what each side is.
fn paint_divider(painter: &egui::Painter, canvas: Rect, x: f32, compare: Compare) {
    painter.vline(x, canvas.y_range(), Stroke::new(2.0, color::FORGE_BRIGHT));
    let grip = Rect::from_center_size(Pos2::new(x, canvas.center().y), Vec2::new(8.0, 34.0));
    painter.rect_filled(grip, 3.0, color::FORGE_BRIGHT);
    let without = match compare {
        Compare::Unlit => "Unlit",
        Compare::Flat | Compare::Off => "Without map",
    };
    for (label_text, at, align) in [
        (without, x - 10.0, egui::Align2::RIGHT_TOP),
        ("Normal mapped", x + 10.0, egui::Align2::LEFT_TOP),
    ] {
        label(
            painter,
            Pos2::new(at, canvas.top() + 10.0),
            align,
            label_text,
            text::LABEL,
            color::TEXT,
        );
    }
}

/// The light's color as a swatch the marker is filled with.
fn swatch(rgb: [f32; 4]) -> Color32 {
    let byte = |value: f32| {
        // Clamped into 0..=255 first.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let byte = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        byte
    };
    Color32::from_rgb(byte(rgb[0]), byte(rgb[1]), byte(rgb[2]))
}

/// The light where it stands: its reach on the plane, a stem from the plane
/// up to its height when the plane is seen at an angle, and the lamp.
fn paint_light(painter: &egui::Painter, canvas: Rect, seen: &ViewCamera, studio: &NormalStudio) {
    let light = &studio.light;
    let [x, y] = light.position;
    let ground = Vec3::new(x, y, 0.0);
    let lamp = Vec3::new(x, y, light.light.height.max(0.0));
    let project = |point: Vec3| project_point(canvas, seen.view_projection, point);
    let tint = swatch(light.light.color);
    let ring: Vec<Pos2> = (0..=64_u8)
        .filter_map(|step| {
            let angle = f32::from(step) / 64.0 * std::f32::consts::TAU;
            project(ground + Vec3::new(angle.cos(), angle.sin(), 0.0) * light.light.radius)
        })
        .collect();
    painter.add(egui::Shape::dashed_line(
        &ring,
        Stroke::new(1.0, tint.gamma_multiply(0.6)),
        6.0,
        5.0,
    ));
    let (Some(foot), Some(top)) = (project(ground), project(lamp)) else {
        return;
    };
    if foot.distance(top) > 2.0 {
        painter.add(egui::Shape::dashed_line(
            &[foot, top],
            Stroke::new(1.5, tint),
            4.0,
            3.0,
        ));
        painter.circle_stroke(foot, 3.0, Stroke::new(1.0, tint));
    }
    painter.circle_filled(top, 7.0, tint);
    painter.circle_stroke(top, 7.0, Stroke::new(1.5, Color32::WHITE));
}

/// What the canvas is showing, and how to move things on it.
fn readout(painter: &egui::Painter, canvas: Rect, studio: &NormalStudio) {
    let Some(picture) = studio.picture() else {
        return;
    };
    let cells = match studio.cell_count() {
        0 => String::new(),
        count => format!("  ·  {count} cells kept apart"),
    };
    let hint = match studio.view.mode {
        StudioMode::Lit => "Drag to move the light",
        StudioMode::Inspect => "Drag to orbit, right-drag to move the light",
        StudioMode::Texture | StudioMode::Normal => "Right-drag to pan",
    };
    let line = format!(
        "{} × {} px{cells}  ·  {hint}, scroll to zoom, double-click to reset",
        picture.width, picture.height
    );
    label(
        painter,
        canvas.left_bottom() + Vec2::new(10.0, -10.0),
        egui::Align2::LEFT_BOTTOM,
        &line,
        text::NOTE,
        color::TEXT_MUTED,
    );
}

/// Words on a dark wash, so they read over any picture.
fn label(
    painter: &egui::Painter,
    at: Pos2,
    align: egui::Align2,
    words: &str,
    size: f32,
    tint: Color32,
) {
    let galley = painter.layout_no_wrap(words.to_owned(), egui::FontId::proportional(size), tint);
    let rect = align.anchor_size(at, galley.size());
    painter.rect_filled(rect.expand(4.0), 3.0, color::SCRIM);
    painter.galley(rect.min, galley, tint);
}
