//! The slicer's controls: how a sheet is divided, and what its cells are
//! called. Drawn beside the image in the Sprite sheet tab.

use eframe::egui;

use crate::slicer::Slicer;
use crate::ui::icons;
use crate::ui::theme::metric;
use crate::ui::widgets::{panel, property, section};

use super::inspector_panel::rows::number_row;

/// The heading above one section of the inspector.
///
/// A collapse chevron and an overflow menu used to sit at either end of it.
/// Neither was handled: nothing collapsed and nothing overflowed. Adding and
/// removing a component is what the menu would hold, and that is a real build
/// against the schema registry rather than a glyph.
/// A pixel measurement, as a drag leaves it.
///
/// Clamped to something an image could plausibly carry, for the reason a grid
/// side is: a drag that got away should not become a slice with no cells.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn pixel_count(value: f64) -> u32 {
    value.clamp(0.0, 4096.0) as u32
}

/// One side of a slicing grid, as a drag leaves it.
///
/// Clamped rather than validated after the fact: a grid of zero has no cells
/// and one of ten thousand is a drag that got away, and neither is a slice
/// anybody meant. The cast cannot lose anything once the value is inside that
/// range.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(super) fn grid_side(value: f64) -> u32 {
    value.clamp(1.0, 256.0) as u32
}

/// The numbers that decide where the cells fall.
///
/// Six drags in one place rather than scattered through the panel: they are one
/// idea — how the sheet is divided — and every one of them moves every cell.
pub(super) fn slice_grid(ui: &mut egui::Ui, slicer: &mut Slicer) {
    section::group(ui, icons::TILEMAP, "Slice");
    let mut columns = f64::from(slicer.columns);
    let mut rows = f64::from(slicer.rows);
    let mut resized = number_row(ui, "Columns", &mut columns, 10.0, true);
    resized |= number_row(ui, "Rows", &mut rows, 10.0, true);
    if resized {
        slicer.columns = grid_side(columns);
        slicer.rows = grid_side(rows);
        slicer.fit_names();
        slicer.clamp_selection();
    }

    let mut margin_x = f64::from(slicer.margin[0]);
    let mut margin_y = f64::from(slicer.margin[1]);
    let mut spacing_x = f64::from(slicer.spacing[0]);
    let mut spacing_y = f64::from(slicer.spacing[1]);
    let mut measured = number_row(ui, "Margin X", &mut margin_x, 10.0, true);
    measured |= number_row(ui, "Margin Y", &mut margin_y, 10.0, true);
    measured |= number_row(ui, "Spacing X", &mut spacing_x, 10.0, true);
    measured |= number_row(ui, "Spacing Y", &mut spacing_y, 10.0, true);
    if measured {
        slicer.margin = [pixel_count(margin_x), pixel_count(margin_y)];
        slicer.spacing = [pixel_count(spacing_x), pixel_count(spacing_y)];
    }
}

/// Naming the chosen cell, and a list of the ones already named.
///
/// A field per cell is fine at four and unusable at two hundred and fifty-six,
/// so the sheet is named the way it is looked at: pick a cell on the image, give
/// it a name. Everything unnamed already has an answer — its index — so a list
/// of the named ones is the whole of what there is to review.
pub(super) fn slice_names(ui: &mut egui::Ui, slicer: &mut Slicer) {
    section::group(ui, icons::LABEL, "Names");
    slicer.fit_names();
    slicer.clamp_selection();

    let selected = slicer.selected;
    let placeholder = selected.to_string();
    if let Some(name) = slicer.names.get_mut(selected as usize) {
        property::Property::new(&format!("Cell {selected}")).show(ui, |ui| {
            ui.add_sized(
                [property::value_width(ui), metric::CONTROL_HEIGHT],
                egui::TextEdit::singleline(name).hint_text(&placeholder),
            );
        });
    }
    section::caption(
        ui,
        "Click a cell on the image to name it. A cell left blank is called by its index.",
    );

    let named = slicer.named();
    if named.is_empty() {
        return;
    }
    ui.add_space(6.0);
    panel::note(ui, &format!("{} named", named.len()));
    let mut jump = None;
    for (index, name) in named {
        // A named cell is a row that jumps to it, drawn with the tree's own
        // banding so it reads as a list of things rather than as a paragraph.
        let row = crate::ui::widgets::tree::row(
            ui,
            icons::LABEL,
            &format!("{index}  ·  {name}"),
            crate::ui::widgets::tree::RowStyle {
                selected: index == selected,
                depth: 1,
                ..crate::ui::widgets::tree::RowStyle::default()
            },
        );
        if row.select.clicked() {
            jump = Some(index);
        }
    }
    // The list is also how a cell is found again on a sheet too large to scan.
    if let Some(jump) = jump {
        slicer.selected = jump;
    }
}
