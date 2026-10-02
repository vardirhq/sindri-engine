//! The controls around the lanes: the toolbar, adding a track, and the
//! picked key's or cue's own fields.

use eframe::egui::{self, RichText};
use egui_material_icons::icons as glyphs;
use sindri_scene::{EASINGS, TRANSFORM_PROPERTIES};

use crate::timeline::{Edit, Picked, TimelineState};
use crate::ui::theme::{color, metric, text};
use crate::ui::widgets::button;

use super::View;

pub(super) fn toolbar(
    ui: &mut egui::Ui,
    state: &mut TimelineState,
    view: &View<'_>,
    edits: &mut Vec<Edit>,
) {
    ui.horizontal(|ui| {
        ui.set_height(metric::TOOLBAR_HEIGHT);
        ui.add_space(metric::GUTTER);
        let mut chosen = view.name.to_owned();
        egui::ComboBox::from_id_salt("timeline sequence")
            .selected_text(view.name)
            .width(110.0)
            .show_ui(ui, |ui| {
                for name in &view.names {
                    ui.selectable_value(&mut chosen, (*name).to_owned(), *name);
                }
            });
        if chosen != view.name {
            state.sequence = Some(chosen);
            state.picked = None;
            state.time = 0.0;
        }
        if button::icon(ui, glyphs::ICON_ADD, false, "Add another sequence").clicked() {
            let mut number = view.names.len() + 1;
            let mut name = format!("sequence {number}");
            while view.names.contains(&name.as_str()) {
                number += 1;
                name = format!("sequence {number}");
            }
            edits.push(Edit::AddSequence(name));
        }
        ui.separator();
        let (glyph, tip) = if state.playing {
            (glyphs::ICON_PAUSE, "Pause the preview")
        } else {
            (
                glyphs::ICON_PLAY_ARROW,
                "Play the preview from the playhead",
            )
        };
        if button::icon(ui, glyph, state.playing, tip).clicked() {
            if !state.playing && state.time >= view.sequence.duration {
                state.time = 0.0;
            }
            state.playing = !state.playing;
            state.preview = true;
        }
        if button::icon(
            ui,
            glyphs::ICON_VISIBILITY,
            state.preview,
            "Show the sequence at the playhead in the Scene view",
        )
        .clicked()
        {
            state.preview = !state.preview;
        }
        ui.label(
            RichText::new(format!("{:.2} s", state.time))
                .size(text::LABEL)
                .monospace()
                .color(color::TEXT),
        );
        ui.separator();
        let mut duration = view.sequence.duration;
        ui.label(
            RichText::new("Length")
                .size(text::LABEL)
                .color(color::TEXT_MUTED),
        );
        if ui
            .add(
                egui::DragValue::new(&mut duration)
                    .speed(0.05)
                    .range(0.05..=600.0)
                    .suffix(" s"),
            )
            .changed()
        {
            edits.push(Edit::Duration(duration));
        }
        let mut looping = view.sequence.looping;
        if ui.checkbox(&mut looping, "Loop").changed() {
            edits.push(Edit::Looping(looping));
        }
        let mut autoplay = view.autoplay == Some(view.name);
        if ui
            .checkbox(&mut autoplay, "Autoplay")
            .on_hover_text("Play this sequence when the scene starts")
            .changed()
        {
            edits.push(Edit::Autoplay(autoplay.then(|| view.name.to_owned())));
        }
    });
}

/// Choosing what a new track moves.
pub(super) fn add_track(
    ui: &mut egui::Ui,
    state: &mut TimelineState,
    view: &View<'_>,
    edits: &mut Vec<Edit>,
) {
    if state.new_property.is_empty() {
        TRANSFORM_PROPERTIES[0].clone_into(&mut state.new_property);
    }
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Add track")
                .size(text::LABEL)
                .color(color::TEXT_MUTED),
        );
        egui::ComboBox::from_id_salt("timeline target")
            .selected_text(if state.new_target.is_empty() {
                "this entity"
            } else {
                state.new_target.as_str()
            })
            .width(120.0)
            .show_ui(ui, |ui| {
                for target in view.targets {
                    let shown = if target.is_empty() {
                        "this entity"
                    } else {
                        target.as_str()
                    };
                    ui.selectable_value(&mut state.new_target, target.clone(), shown);
                }
            });
        egui::ComboBox::from_id_salt("timeline property")
            .selected_text(state.new_property.as_str())
            .width(110.0)
            .show_ui(ui, |ui| {
                for property in TRANSFORM_PROPERTIES {
                    ui.selectable_value(&mut state.new_property, property.to_owned(), property);
                }
            });
        ui.add(
            egui::TextEdit::singleline(&mut state.new_property)
                .desired_width(120.0)
                .hint_text("or component/field"),
        )
        .on_hover_text(
            "A component field, written as the component, a slash and a path: sindri.sprite/tint.3",
        );
        let valid = sindri_scene::Property::parse(&state.new_property).is_ok();
        if ui.add_enabled(valid, egui::Button::new("Add")).clicked() {
            edits.push(Edit::AddTrack {
                target: state.new_target.clone(),
                property: state.new_property.clone(),
            });
        }
    });
}

/// The picked key's or cue's own fields.
pub(super) fn picked_editor(
    ui: &mut egui::Ui,
    state: &mut TimelineState,
    view: &View<'_>,
    edits: &mut Vec<Edit>,
) {
    match state.picked {
        Some(Picked::Key { track, key }) => key_editor(ui, state, view, edits, track, key),
        Some(Picked::Cue(cue)) => cue_editor(ui, state, view, edits, cue),
        None => {
            ui.label(
                RichText::new("Pick a key or a cue to edit it. Drag one to move it in time.")
                    .size(text::NOTE)
                    .color(color::TEXT_FAINT),
            );
        }
    }
}

/// The picked key's time, value and curve.
pub(super) fn key_editor(
    ui: &mut egui::Ui,
    state: &mut TimelineState,
    view: &View<'_>,
    edits: &mut Vec<Edit>,
    track: usize,
    key: usize,
) {
    let duration = view.sequence.duration;
    let Some(found) = view
        .sequence
        .tracks
        .get(track)
        .and_then(|t| t.keys.get(key))
    else {
        state.picked = None;
        return;
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new("Key").size(text::HEADING).color(color::TEXT));
        let mut time = found.time;
        ui.label(
            RichText::new("at")
                .size(text::LABEL)
                .color(color::TEXT_MUTED),
        );
        if ui
            .add(
                egui::DragValue::new(&mut time)
                    .speed(0.01)
                    .range(0.0..=duration)
                    .suffix(" s"),
            )
            .changed()
        {
            edits.push(Edit::MoveKey { track, key, time });
        }
        let mut value = found.value;
        let mut ease = found.ease.clone();
        ui.label(
            RichText::new("value")
                .size(text::LABEL)
                .color(color::TEXT_MUTED),
        );
        let changed = ui
            .add(egui::DragValue::new(&mut value).speed(0.05))
            .changed();
        let mut eased = false;
        egui::ComboBox::from_id_salt("timeline ease")
            .selected_text(ease.as_str())
            .width(96.0)
            .show_ui(ui, |ui| {
                for name in EASINGS {
                    eased |= ui
                        .selectable_value(&mut ease, name.to_owned(), name)
                        .changed();
                }
            })
            .response
            .on_hover_text("How the track moves from this key to the next");
        if changed || eased {
            edits.push(Edit::SetKey {
                track,
                key,
                value,
                ease,
            });
        }
        if button::row_icon(
            ui,
            glyphs::ICON_DELETE,
            button::Intent::Quiet,
            "Remove this key",
        )
        .clicked()
        {
            edits.push(Edit::RemoveKey { track, key });
            state.picked = None;
        }
    });
}

/// The picked cue's name, time and sound.
pub(super) fn cue_editor(
    ui: &mut egui::Ui,
    state: &mut TimelineState,
    view: &View<'_>,
    edits: &mut Vec<Edit>,
    cue: usize,
) {
    let duration = view.sequence.duration;
    let Some(found) = view.sequence.cues.get(cue) else {
        state.picked = None;
        return;
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new("Cue").size(text::HEADING).color(color::TEXT));
        let mut name = found.name.clone();
        let mut time = found.time;
        let mut sound = found
            .sound
            .as_ref()
            .map(|s| s.clip.clone())
            .unwrap_or_default();
        let mut changed = ui
            .add(egui::TextEdit::singleline(&mut name).desired_width(100.0))
            .on_hover_text("What a script asks Sequence.cued for")
            .changed();
        ui.label(
            RichText::new("at")
                .size(text::LABEL)
                .color(color::TEXT_MUTED),
        );
        changed |= ui
            .add(
                egui::DragValue::new(&mut time)
                    .speed(0.01)
                    .range(0.0..=duration)
                    .suffix(" s"),
            )
            .changed();
        ui.label(
            RichText::new("sound")
                .size(text::LABEL)
                .color(color::TEXT_MUTED),
        );
        changed |= ui
            .add(
                egui::TextEdit::singleline(&mut sound)
                    .desired_width(140.0)
                    .hint_text("none"),
            )
            .on_hover_text("A clip to play when the cue is reached, such as audio/thud.wav")
            .changed();
        if changed {
            edits.push(Edit::SetCue {
                cue,
                time,
                name,
                sound,
            });
        }
        if button::row_icon(
            ui,
            glyphs::ICON_DELETE,
            button::Intent::Quiet,
            "Remove this cue",
        )
        .clicked()
        {
            edits.push(Edit::RemoveCue(cue));
            state.picked = None;
        }
    });
}
