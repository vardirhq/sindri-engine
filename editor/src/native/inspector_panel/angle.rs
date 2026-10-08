//! Angles in degrees, and fields that are absent until added.
//!
//! An angle is stored in radians, which is what the engine turns by and what
//! no author thinks in: a hinge's limit of `0.7853982` is a quarter of a half
//! turn, and the row now says 45°. The stored value is still radians, so a
//! scene is byte for byte what it was until the angle is changed.
//!
//! Before either, a described field gets the menu on its name
//! (`section::heading::field_menu`).
//!
//! A field the component declares optional (`describe_optional`) is `null`
//! until added: it reads as not set, with Add writing what the registration
//! says it holds, and once added a Remove that writes `null` back.

use eframe::egui::{self, RichText};
use serde_json::Value;

use super::rows::At;
use crate::ui::theme::{color, metric, text};
use crate::ui::widgets::{button, button::Intent, property};

/// A number in radians, shown and dragged in degrees.
pub(super) fn angle_row(ui: &mut egui::Ui, label: &str, value: &mut Value, indent: f32) {
    let radians = value.as_f64().unwrap_or_default();
    let mut degrees = radians.to_degrees();
    let mut changed = false;
    property::Property::new(label)
        .indent(indent)
        .show(ui, |ui| {
            changed = ui
                .add_sized(
                    [property::value_width(ui), metric::CONTROL_HEIGHT],
                    egui::DragValue::new(&mut degrees)
                        .speed(0.5)
                        .max_decimals(2)
                        .suffix("°"),
                )
                .on_hover_text("In degrees here; stored in radians")
                .changed();
        });
    if changed {
        *value = Value::from(degrees.to_radians());
    }
}

/// An optional field that has not been added: says so, and offers Add.
/// Answers what to write if Add was pressed.
pub(super) fn absent_row(
    ui: &mut egui::Ui,
    label: &str,
    added: &Value,
    indent: f32,
) -> Option<Value> {
    let mut adding = false;
    property::Property::new(label)
        .indent(indent)
        .show(ui, |ui| {
            ui.label(
                RichText::new("Not set")
                    .size(text::LABEL)
                    .color(color::TEXT_FAINT),
            );
            adding = button::labelled(ui, "Add", Intent::Quiet, &format!("Add {label}")).clicked();
        });
    adding.then(|| added.clone())
}

/// The heading over an optional field that has been added, with the way to
/// take it off again. Answers whether Remove was pressed.
pub(super) fn present_heading(ui: &mut egui::Ui, label: &str, indent: f32) -> bool {
    let mut removing = false;
    ui.horizontal(|ui| {
        ui.add_space(metric::GUTTER + indent);
        ui.label(
            RichText::new(label)
                .size(text::LABEL)
                .color(color::TEXT_FAINT),
        );
        removing =
            button::labelled(ui, "Remove", Intent::Quiet, &format!("Remove {label}")).clicked();
    });
    removing
}

/// What a described field gets before its own row: the menu on its name, and
/// if it is optional, its Add or its Remove. Answers whether that drew the
/// whole of it.
pub(super) fn around_row(
    ui: &mut egui::Ui,
    at: At<'_>,
    label: &str,
    value: &mut Value,
    indent: f32,
) -> bool {
    let Some(described) = at.described else {
        return false;
    };
    // A component's own field has a name to right-click; a piece of a list
    // is reached through its list.
    if let Some(written) = super::section::heading::field_menu(
        ui,
        described.type_name,
        at.path,
        label,
        indent,
        value,
        at.exemplar(),
    ) {
        *value = written;
    }
    let Some(added) = described.registry.optional(described.type_name, at.path) else {
        return false;
    };
    if value.is_null() {
        if let Some(added) = absent_row(ui, label, added, indent) {
            *value = added;
        }
        return true;
    }
    if value.is_object() {
        if present_heading(ui, label, indent) {
            *value = Value::Null;
        } else {
            super::rows::object_rows(ui, at, value, indent);
        }
        return true;
    }
    false
}
