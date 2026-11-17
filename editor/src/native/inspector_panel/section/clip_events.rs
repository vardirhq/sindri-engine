//! A clip's events: named moments a frame fires as it is shown, a foot landing
//! or a blade striking, which a script hears with `Animation.fired`.
//!
//! Edited in the stored payload, as the rest of the clip is, so the draft's
//! diff makes each change one undoable edit.

use eframe::egui::{self, RichText};
use serde_json::{Value, json};
use sindri_scene::AnimationClip;

use crate::ui::icons;
use crate::ui::theme::{color, metric, text};
use crate::ui::widgets::{
    button::{self, Intent},
    property, section,
};

/// The selected clip's events, each a frame and a name, with add and remove.
pub(super) fn clip_events(
    ui: &mut egui::Ui,
    payload: &mut Value,
    clip_name: &str,
    clip: &AnimationClip,
) {
    section::group(ui, icons::SPRITE, "Events");
    let last_frame = clip.frames.len().saturating_sub(1);
    let mut events = clip.events.clone();
    let mut removed = None;
    let mut changed = false;
    for (index, event) in events.iter_mut().enumerate() {
        property::Property::new(&format!("Frame {}", event.frame + 1)).show(ui, |ui| {
            let mut shown = event.frame + 1;
            changed |= ui
                .add(
                    egui::DragValue::new(&mut shown)
                        .range(1..=last_frame + 1)
                        .speed(0.1),
                )
                .on_hover_text("The frame that fires it, counted from 1")
                .changed();
            event.frame = shown.saturating_sub(1).min(last_frame);
            let width = (property::value_width(ui) - 60.0).max(60.0);
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut event.name)
                        .desired_width(width)
                        .hint_text("step"),
                )
                .on_hover_text("The name a script asks for: Animation.fired(entity, \"step\")")
                .changed();
            if button::row_icon(ui, icons::REMOVE, Intent::Danger, "Remove this event").clicked() {
                removed = Some(index);
            }
        });
    }
    if let Some(index) = removed {
        events.remove(index);
        changed = true;
    }
    ui.horizontal(|ui| {
        ui.add_space(metric::GUTTER);
        if ui
            .button(
                RichText::new("Add event")
                    .size(text::LABEL)
                    .color(color::TEXT_MUTED),
            )
            .on_hover_text("A name this clip fires as one of its frames is shown")
            .clicked()
        {
            events.push(sindri_scene::ClipEvent {
                frame: 0,
                name: "step".to_owned(),
            });
            changed = true;
        }
    });
    if changed && let Some(stored) = payload["clips"].get_mut(clip_name) {
        if events.is_empty() {
            if let Some(object) = stored.as_object_mut() {
                object.remove("events");
            }
        } else {
            stored["events"] = json!(events);
        }
    }
}
