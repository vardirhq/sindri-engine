//! One `@export` value, drawn as what its script says it is: a variant picked
//! by name, a profile picked from the project, a list whose items are added
//! and removed, a struct whose fields each get a row — and anything else as
//! the plain value it is.

use eframe::egui::{self, RichText};
use serde_json::Value;
use sindri_decay::{ScriptExport, ScriptValue};

use crate::ui::icons;
use crate::ui::theme::{color, metric, text};
use crate::ui::widgets::{button, button::Intent};

use super::super::rows::{At, Authored, value_row};

/// What the author asked to happen to a list, decided while drawing it and
/// applied after, so an item is never removed from under the loop drawing it.
#[derive(Clone, Copy)]
enum Change {
    Add,
    Remove(usize),
    MoveUp(usize),
}

/// One value. `id` tells this row's pickers apart from every other's: two
/// items of one list would otherwise share a dropdown.
pub(super) fn export_row(
    ui: &mut egui::Ui,
    id: &str,
    export: &ScriptExport,
    value: &mut Value,
    indent: f32,
    authored: Authored,
    profiles: &[String],
) {
    if let Some(element) = &export.element {
        list_rows(ui, id, export, element, value, indent, profiles);
    } else if !export.fields.is_empty() {
        struct_rows(ui, id, export, value, indent, profiles);
    } else if !export.choices.is_empty() {
        // An enum: its variants, by name, never a number.
        let current = value.as_str().unwrap_or_default().to_owned();
        if let Some(chosen) =
            super::super::field::named_choice_row(ui, id, &export.name, &current, &export.choices)
        {
            *value = Value::String(chosen);
        }
    } else if export.type_name.as_deref() == Some("Profile") {
        super::super::field::asset_row(ui, id, &export.name, value, profiles, None, indent);
    } else {
        value_row(ui, At::loose(), &export.name, value, indent, authored);
    }
}

fn heading(ui: &mut egui::Ui, label: &str, indent: f32) {
    ui.horizontal(|ui| {
        ui.add_space(metric::GUTTER + indent);
        ui.label(
            RichText::new(crate::inspector::humanize(label))
                .size(text::LABEL)
                .color(color::TEXT_FAINT),
        );
    });
}

/// A struct: its name as a heading, then a row per field, in declared order.
/// A field the scene has not written shows the script's default.
fn struct_rows(
    ui: &mut egui::Ui,
    id: &str,
    export: &ScriptExport,
    value: &mut Value,
    indent: f32,
    profiles: &[String],
) {
    if !export.name.is_empty() {
        heading(ui, &export.name, indent);
    }
    if !value.is_object() {
        *value = script_value_json(&export.default);
    }
    let Value::Object(fields) = value else {
        return;
    };
    for field in &export.fields {
        let entry = fields
            .entry(field.name.clone())
            .or_insert_with(|| script_value_json(&field.default));
        export_row(
            ui,
            &format!("{id}.{}", field.name),
            field,
            entry,
            indent + 10.0,
            Authored::Default,
            profiles,
        );
    }
}

/// A list: a heading that counts and adds, then each item with its own
/// controls and its rows beneath.
fn list_rows(
    ui: &mut egui::Ui,
    id: &str,
    export: &ScriptExport,
    element: &ScriptExport,
    value: &mut Value,
    indent: f32,
    profiles: &[String],
) {
    if !value.is_array() {
        *value = script_value_json(&export.default);
        if !value.is_array() {
            *value = Value::Array(Vec::new());
        }
    }
    let mut change = None;
    let count = value.as_array().map_or(0, Vec::len);
    ui.horizontal(|ui| {
        ui.add_space(metric::GUTTER + indent);
        ui.label(
            RichText::new(crate::inspector::humanize(&export.name))
                .size(text::LABEL)
                .color(color::TEXT_FAINT),
        );
        ui.label(
            RichText::new(format!("{count}"))
                .size(text::NOTE)
                .color(color::TEXT_FAINT),
        );
        if button::row_icon(ui, icons::ADD, Intent::Quiet, "Add one").clicked() {
            change = Some(Change::Add);
        }
    });
    let Some(items) = value.as_array_mut() else {
        return;
    };
    for (index, item) in items.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.add_space(metric::GUTTER + indent + 10.0);
            ui.label(
                RichText::new(format!("{}", index + 1))
                    .size(text::LABEL)
                    .color(color::TEXT_FAINT),
            );
            if index > 0 && button::row_icon(ui, icons::MOVE_UP, Intent::Quiet, "Move up").clicked()
            {
                change = Some(Change::MoveUp(index));
            }
            if button::row_icon(ui, icons::REMOVE, Intent::Quiet, "Remove").clicked() {
                change = Some(Change::Remove(index));
            }
        });
        export_row(
            ui,
            &format!("{id}[{index}]"),
            element,
            item,
            indent + 20.0,
            Authored::Default,
            profiles,
        );
    }
    apply(script_value_json(&element.default), items, change);
}

/// A script's list may be emptied — it is the script's to have none — unlike
/// a component's pieces the engine requires at least one of.
fn apply(blank: Value, items: &mut Vec<Value>, change: Option<Change>) {
    match change {
        Some(Change::Add) => items.push(blank),
        Some(Change::Remove(index)) if index < items.len() => {
            items.remove(index);
        }
        Some(Change::MoveUp(index)) if index > 0 && index < items.len() => {
            items.swap(index - 1, index);
        }
        _ => {}
    }
}

/// A Decay value as the JSON a scene stores.
///
/// A reference stores as null, because it names a runtime handle and runtime
/// handles are never serialized: writing one to a scene would produce a file
/// that means something different the next time it is opened. An `@export` of
/// an entity is not authorable for that reason, and the inspector shows it as
/// empty rather than as a number nobody can act on.
///
/// A list stores as its items and a struct as an object of its fields, each
/// the same way, so what a scene holds is what the script declared.
pub(super) fn script_value_json(value: &ScriptValue) -> Value {
    match value {
        ScriptValue::Number(number) => Value::from(*number),
        ScriptValue::Bool(flag) => Value::Bool(*flag),
        ScriptValue::String(text) => Value::String(text.clone()),
        // As its components, the way a transform stores a position, so the
        // same vector row edits it.
        ScriptValue::Vec2(components) => Value::from(components.to_vec()),
        ScriptValue::Vec3(components) => Value::from(components.to_vec()),
        // By the variant's own name, as a scene authors one.
        ScriptValue::Variant(name) => Value::String(
            name.split_once('.')
                .map_or(&**name, |(_, variant)| variant)
                .to_owned(),
        ),
        ScriptValue::Array(items) => Value::Array(items.iter().map(script_value_json).collect()),
        ScriptValue::Struct { shape, fields } => Value::Object(
            shape
                .fields
                .iter()
                .zip(fields.iter())
                .map(|(name, field)| (name.clone(), script_value_json(field)))
                .collect(),
        ),
        // A timer is started by the script, not authored: it runs down in
        // play, and a number in the inspector would be stale on the next frame.
        // A map is not authored by a scene yet, so it stores as empty too.
        ScriptValue::Reference(_)
        | ScriptValue::Timer { .. }
        | ScriptValue::Map(_)
        | ScriptValue::Null
        | ScriptValue::Unit => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Change, apply};

    #[test]
    fn a_script_list_grows_shrinks_and_reorders_down_to_nothing() {
        let mut items = vec![json!(1.0)];
        apply(json!(0.0), &mut items, Some(Change::Add));
        assert_eq!(items, [json!(1.0), json!(0.0)]);
        apply(json!(0.0), &mut items, Some(Change::MoveUp(1)));
        assert_eq!(items, [json!(0.0), json!(1.0)]);
        apply(json!(0.0), &mut items, Some(Change::Remove(0)));
        apply(json!(0.0), &mut items, Some(Change::Remove(0)));
        assert!(items.is_empty());
        apply(json!(0.0), &mut items, Some(Change::Remove(3)));
        assert!(items.is_empty());
    }
}
