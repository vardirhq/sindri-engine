//! The nine-slice guides drawn over the picked cell, and dragging them.
//!
//! Four dashed lines inset from the cell's edges by its border. Dragging one
//! with the primary button moves that side of the border a whole pixel at a
//! time; a click still picks a cell, because a guide only takes a drag that
//! started on it.

use eframe::egui::{self, Pos2, Rect, Stroke};

use crate::slicer::Slicer;
use crate::ui::theme::color;

/// How close, in screen pixels, a press must be to take a guide.
const REACH: f32 = 6.0;

/// Draws the picked cell's guides and moves the one being dragged.
pub(super) fn border_guides(
    ui: &egui::Ui,
    response: &egui::Response,
    painter: &egui::Painter,
    slicer: &mut Slicer,
    cell: Rect,
    zoom: f32,
) {
    let border = slicer.border();
    let lines = guide_positions(cell, border, zoom);
    let id = response.id.with("nine-slice guide");
    let hovered = response
        .hover_pos()
        .or_else(|| response.interact_pointer_pos());
    let near = hovered.and_then(|at| nearest(cell, lines, at));
    if response.drag_started_by(egui::PointerButton::Primary) {
        ui.data_mut(|data| data.insert_temp(id, near));
    }
    let held: Option<usize> = ui.data(|data| data.get_temp(id)).flatten();
    if let Some(side) = held.or(near) {
        ui.ctx().set_cursor_icon(if side.is_multiple_of(2) {
            egui::CursorIcon::ResizeHorizontal
        } else {
            egui::CursorIcon::ResizeVertical
        });
    }
    if let (Some(side), Some(at)) = (held, response.interact_pointer_pos())
        && response.dragged_by(egui::PointerButton::Primary)
    {
        slicer.set_border(dragged(border, side, cell, at, zoom));
    }
    if response.drag_stopped() {
        ui.data_mut(|data| data.remove::<Option<usize>>(id));
    }
    draw(
        painter,
        cell,
        guide_positions(cell, slicer.border(), zoom),
        held.or(near),
    );
}

/// Where each guide sits on screen: left and right as x, top and bottom as y,
/// in the border's own order.
fn guide_positions(cell: Rect, border: [u32; 4], zoom: f32) -> [f32; 4] {
    #[allow(clippy::cast_precision_loss)]
    let [left, top, right, bottom] = border.map(|side| side as f32 * zoom);
    [
        cell.left() + left,
        cell.top() + top,
        cell.right() - right,
        cell.bottom() - bottom,
    ]
}

/// The guide under `at`, if one is within reach and `at` is over the cell.
fn nearest(cell: Rect, lines: [f32; 4], at: Pos2) -> Option<usize> {
    if !cell.expand(REACH).contains(at) {
        return None;
    }
    (0..4_usize)
        .map(|side| {
            let along = if side.is_multiple_of(2) { at.x } else { at.y };
            (side, (along - lines[side]).abs())
        })
        .filter(|(_, distance)| *distance <= REACH)
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(side, _)| side)
}

/// The border with `side` moved to the pixel under `at`, kept from crossing
/// the opposite guide.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn dragged(mut border: [u32; 4], side: usize, cell: Rect, at: Pos2, zoom: f32) -> [u32; 4] {
    let span = if side.is_multiple_of(2) {
        cell.width()
    } else {
        cell.height()
    } / zoom;
    let opposite = border[(side + 2) % 4];
    let inset = match side {
        0 => at.x - cell.left(),
        1 => at.y - cell.top(),
        2 => cell.right() - at.x,
        _ => cell.bottom() - at.y,
    } / zoom;
    #[allow(clippy::cast_precision_loss)]
    let room = (span.round() - opposite as f32).max(0.0);
    border[side] = inset.round().clamp(0.0, room) as u32;
    border
}

fn draw(painter: &egui::Painter, cell: Rect, lines: [f32; 4], hot: Option<usize>) {
    for (side, line) in lines.into_iter().enumerate() {
        let stroke = Stroke::new(
            if hot == Some(side) { 2.0 } else { 1.0 },
            color::FORGE_BRIGHT,
        );
        let [from, to] = if side.is_multiple_of(2) {
            [Pos2::new(line, cell.top()), Pos2::new(line, cell.bottom())]
        } else {
            [Pos2::new(cell.left(), line), Pos2::new(cell.right(), line)]
        };
        painter.extend(egui::Shape::dashed_line(&[from, to], stroke, 4.0, 3.0));
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Pos2, Rect};

    use super::{dragged, guide_positions, nearest};

    fn cell() -> Rect {
        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(96.0, 96.0))
    }

    /// At four times, a six-pixel border sits 24 screen pixels in, and a press
    /// near that line takes it.
    #[test]
    fn a_guide_is_taken_near_its_line() {
        let lines = guide_positions(cell(), [6, 6, 6, 6], 4.0);
        assert!(
            lines
                .iter()
                .zip([24.0, 24.0, 72.0, 72.0])
                .all(|(got, want)| (got - want).abs() < 1.0e-4)
        );
        assert_eq!(nearest(cell(), lines, Pos2::new(26.0, 50.0)), Some(0));
        assert_eq!(nearest(cell(), lines, Pos2::new(50.0, 70.0)), Some(3));
        assert_eq!(nearest(cell(), lines, Pos2::new(50.0, 50.0)), None);
    }

    /// Dragging lands on whole pixels and cannot cross the opposite guide.
    #[test]
    fn a_dragged_guide_snaps_and_stops_at_its_opposite() {
        let moved = dragged([6, 6, 6, 6], 0, cell(), Pos2::new(41.0, 0.0), 4.0);
        assert_eq!(moved[0], 10);
        let crossed = dragged([6, 6, 6, 6], 0, cell(), Pos2::new(95.0, 0.0), 4.0);
        assert_eq!(crossed[0], 18, "24 pixels wide, less the right border");
    }
}
