//! A collision mask, chosen by the names the scene's physics world gives its
//! layers rather than typed as a number.
//!
//! The stored value stays the number physics reads. A bit the world does not
//! name is kept as it is and counted, so a mask written before its layers had
//! names is not quietly narrowed by opening it here.

use std::fmt::Write as _;

use eframe::egui::{self, RichText};
use serde_json::Value;

use crate::ui::theme::{color, text};
use crate::ui::widgets::property;

/// What a mask holds, in words: the named layers it has, or everything or
/// nothing, and how many unnamed bits besides.
pub(crate) fn summary(mask: u32, layers: &[String]) -> String {
    if mask == u32::MAX {
        return "Everything".to_owned();
    }
    if mask == 0 {
        return "Nothing".to_owned();
    }
    let mut named = Vec::new();
    let mut unnamed = mask;
    for (bit, name) in layers.iter().enumerate().take(32) {
        let flag = 1_u32 << bit;
        if !name.is_empty() && mask & flag != 0 {
            named.push(name.as_str());
            unnamed &= !flag;
        }
    }
    let others = unnamed.count_ones();
    let mut said = named.join(", ");
    if others > 0 {
        if !said.is_empty() {
            said.push_str(" + ");
        }
        let _ = write!(
            said,
            "{others} unnamed layer{}",
            if others == 1 { "" } else { "s" }
        );
    }
    said
}

/// The mask row: a menu of the named layers, each switched on or off, with
/// everything and nothing as shortcuts. Falls back to the plain number when
/// the world names no layers, since a menu of nothing is no help.
pub(crate) fn mask_row(
    ui: &mut egui::Ui,
    at: &str,
    label: &str,
    value: &mut Value,
    layers: &[String],
    indent: f32,
) -> bool {
    let Some(mask) = value.as_u64().and_then(|n| u32::try_from(n).ok()) else {
        return false;
    };
    if layers.iter().all(String::is_empty) {
        return false;
    }
    let mut chosen = mask;
    property::Property::new(label)
        .indent(indent)
        .show(ui, |ui| {
            egui::ComboBox::from_id_salt(("mask", at))
                .selected_text(
                    RichText::new(summary(mask, layers))
                        .size(text::LABEL)
                        .color(color::TEXT_MUTED),
                )
                .width(property::picker_width(ui))
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(chosen == u32::MAX, "Everything")
                        .clicked()
                    {
                        chosen = u32::MAX;
                    }
                    if ui.selectable_label(chosen == 0, "Nothing").clicked() {
                        chosen = 0;
                    }
                    ui.separator();
                    for (bit, name) in layers.iter().enumerate().take(32) {
                        if name.is_empty() {
                            continue;
                        }
                        let flag = 1_u32 << bit;
                        let mut on = chosen & flag != 0;
                        if ui.checkbox(&mut on, name.as_str()).changed() {
                            chosen = if on { chosen | flag } else { chosen & !flag };
                        }
                    }
                });
        });
    if chosen != mask {
        *value = Value::from(chosen);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::summary;

    #[test]
    fn a_mask_is_said_by_its_layer_names() {
        let layers = ["ground", "hero", "pickups"].map(str::to_owned);
        assert_eq!(summary(u32::MAX, &layers), "Everything");
        assert_eq!(summary(0, &layers), "Nothing");
        assert_eq!(summary(0b101, &layers), "ground, pickups");
        assert_eq!(summary(0b1_0010, &layers), "hero + 1 unnamed layer");
        assert_eq!(summary(0b11_0000, &layers), "2 unnamed layers");
    }
}
