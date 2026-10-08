//! The menu on a component's heading, and on a field's name.
//!
//! What acts on a whole component, or a whole field, without a control of
//! its own in the row: copying its values to paste onto another entity's
//! component of the same kind, putting it back as the schema makes it, and
//! removing it. The order components are listed in is not authored (they are
//! sorted, so every entity lists the same kinds in the same place), so there
//! is no Move up or Move down to offer.
//!
//! These edit the draft the inspector is drawing, like every control in it,
//! so each lands as one ordinary edit, undoable, and recorded against a run
//! while one plays.

use eframe::egui::{self, Rect, Sense, Vec2};
use serde_json::Value;
use sindri_core::ComponentSchemaRegistry;

use crate::ui::theme::metric;
use crate::ui::widgets::menu;

/// Where copied component values wait to be pasted: the component's type and
/// its payload, kept in egui's memory for as long as the editor runs.
const COPIED_COMPONENT: &str = "sindri-copied-component";

/// What a component heading's menu asked for.
pub(super) enum HeadingAsk {
    Copy,
    Paste(Value),
    Reset(Value),
    Remove,
}

/// The heading menu's entries for the component `name`, whose payload is
/// `payload`.
pub(super) fn heading_menu(
    ui: &mut egui::Ui,
    name: &str,
    title: &str,
    removable: bool,
    registry: &ComponentSchemaRegistry,
    asked: &mut Option<HeadingAsk>,
) {
    menu::subject(ui, title);
    if menu::item(ui, "Copy values").clicked() {
        *asked = Some(HeadingAsk::Copy);
        ui.close();
    }
    let copied = ui
        .ctx()
        .data(|data| data.get_temp::<(String, Value)>(egui::Id::new(COPIED_COMPONENT)))
        .filter(|(kind, _)| kind == name)
        .map(|(_, value)| value);
    let paste = ui
        .add_enabled(copied.is_some(), menu::entry("Paste values", ""))
        .on_disabled_hover_text("Copy the values of a component of this kind first");
    if paste.clicked()
        && let Some(value) = copied
    {
        *asked = Some(HeadingAsk::Paste(value));
        ui.close();
    }
    let blank = registry.default_payload(name).cloned();
    let reset = ui
        .add_enabled(blank.is_some(), menu::entry("Reset to default", ""))
        .on_disabled_hover_text(
            "Its blank names an asset from the project, so reset its fields one at a time",
        );
    if reset.clicked()
        && let Some(blank) = blank
    {
        *asked = Some(HeadingAsk::Reset(blank));
        ui.close();
    }
    if removable {
        ui.separator();
        if menu::danger(ui, "Remove component", "").clicked() {
            *asked = Some(HeadingAsk::Remove);
            ui.close();
        }
    }
}

/// Carries out what the heading's menu asked for on the draft payload,
/// answering whether the component is to be removed.
pub(super) fn act_on_heading(
    ui: &egui::Ui,
    name: &str,
    payload: &mut Value,
    asked: HeadingAsk,
) -> bool {
    match asked {
        HeadingAsk::Copy => {
            ui.ctx().data_mut(|data| {
                data.insert_temp(
                    egui::Id::new(COPIED_COMPONENT),
                    (name.to_owned(), payload.clone()),
                );
            });
            // And as text, which is what pasting it into a bug report or a
            // scene file by hand wants.
            if let Ok(text) = serde_json::to_string_pretty(payload) {
                ui.ctx().copy_text(text);
            }
            false
        }
        HeadingAsk::Paste(value) | HeadingAsk::Reset(value) => {
            *payload = value;
            false
        }
        HeadingAsk::Remove => true,
    }
}

/// A right-click on the name of the field about to be drawn: copying its
/// value, and putting it back as the schema makes it.
///
/// Laid under the row before it is drawn, so the controls drawn over it keep
/// every click of their own; the name is a label, which takes none, so a
/// right-click on it reaches this. Answers what to write over the value.
pub(crate) fn field_menu(
    ui: &mut egui::Ui,
    component: &str,
    path: &str,
    label: &str,
    indent: f32,
    value: &Value,
    default: Option<&Value>,
) -> Option<Value> {
    let width = (metric::LABEL_WIDTH - indent).max(52.0) + 8.0;
    let rect = Rect::from_min_size(
        ui.cursor().min + Vec2::new(metric::GUTTER + indent, 0.0),
        Vec2::new(width, metric::CONTROL_HEIGHT),
    );
    let response = ui.interact(
        rect,
        ui.id().with(("field-menu", component, path)),
        Sense::click(),
    );
    let mut written = None;
    menu::on_right_click(&response, |ui| {
        menu::subject(ui, label);
        if menu::item(ui, "Copy value").clicked() {
            let text = match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            ui.ctx().copy_text(text);
            ui.close();
        }
        let resettable = default.is_some_and(|default| default != value);
        if ui
            .add_enabled(resettable, menu::entry("Reset to default", ""))
            .clicked()
        {
            written = default.cloned();
            ui.close();
        }
    });
    written
}
