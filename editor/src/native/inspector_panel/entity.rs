//! Entity fields remain typeable, with scoped choices and visible diagnostics.
use eframe::egui::{self, RichText};
use serde_json::Value;

use crate::inspector::entities::{EntityReferences, ReferenceStatus};
use crate::ui::theme::{color, metric, text};
use crate::ui::widgets::property;

pub(super) fn entity_row(
    ui: &mut egui::Ui,
    at: &str,
    label: &str,
    value: &mut Value,
    entities: &EntityReferences,
    indent: f32,
) -> egui::Response {
    let mut typed = value.as_str().unwrap_or_default().to_owned();
    let status = entities.status(&typed);
    let tip = match &status {
        ReferenceStatus::Unbound => "No entity selected".to_owned(),
        ReferenceStatus::Missing => "No entity resolves in this scene or prefab".to_owned(),
        ReferenceStatus::Inactive(name) => format!("{name} is inactive"),
        ReferenceStatus::Active(name) => format!("References {name}"),
    };
    let mut changed = false;
    let picker_response = property::Property::new(label)
        .indent(indent)
        .tip(&tip)
        .show(ui, |ui| {
            let picker = 12.0;
            let width =
                (property::value_width(ui) - picker - property::PICKER_FURNITURE - 4.0).max(56.0);
            let response = ui
                .scope(|ui| {
                    if matches!(
                        status,
                        ReferenceStatus::Missing | ReferenceStatus::Inactive(_)
                    ) {
                        ui.visuals_mut().override_text_color = Some(color::DANGER);
                    }
                    ui.add_sized(
                        [width, metric::CONTROL_HEIGHT],
                        egui::TextEdit::singleline(&mut typed).hint_text("None"),
                    )
                })
                .inner;
            changed |= response.changed();
            response.on_hover_text(&tip);
            egui::ComboBox::from_id_salt(("entity-reference", at))
                .selected_text("")
                .width(picker)
                .show_ui(ui, |ui| {
                    ui.set_min_width(240.0);
                    changed |= ui
                        .selectable_value(&mut typed, String::new(), "None")
                        .changed();
                    for choice in &entities.choices {
                        let inactive = matches!(
                            entities.status(&choice.reference),
                            ReferenceStatus::Inactive(_)
                        );
                        let label = if inactive {
                            format!("{} — inactive", choice.label)
                        } else {
                            choice.label.clone()
                        };
                        if ui
                            .selectable_label(typed == choice.reference, label)
                            .clicked()
                        {
                            typed.clone_from(&choice.reference);
                            changed = true;
                            ui.close();
                        }
                    }
                })
                .response
                .on_hover_text("Choose an entity in this scene or prefab")
        });
    if matches!(
        status,
        ReferenceStatus::Missing | ReferenceStatus::Inactive(_)
    ) {
        ui.horizontal(|ui| {
            ui.add_space(metric::GUTTER + indent);
            ui.label(RichText::new(&tip).size(text::NOTE).color(color::DANGER));
        });
    }
    if changed {
        *value = Value::String(typed);
    }
    picker_response
}

#[cfg(test)]
#[path = "entity_tests.rs"]
mod tests;
