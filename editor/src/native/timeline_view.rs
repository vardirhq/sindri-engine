//! The Timeline panel: the selected entity's sequences as rows of keys
//! against time, with a playhead that poses the Scene view.

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use egui_material_icons::icons as glyphs;
use sindri_core::{CommandBuffer, SceneComponent, WorldCommand};
use sindri_scene::{EASINGS, Sequence, SequenceComponent, TRANSFORM_PROPERTIES};

use crate::timeline::{Edit, Picked, TimelineState, apply, current_value, snapped, targets};
use crate::ui::icons;
use crate::ui::theme::{color, metric, text};
use crate::ui::widgets::{button, panel};

use super::EditorApp;

const LABEL_WIDTH: f32 = 170.0;
const ROW: f32 = 24.0;
const RULER: f32 = 22.0;

/// What the panel draws from, worked out before it draws.
pub(super) struct View<'a> {
    pub name: &'a str,
    pub sequence: &'a Sequence,
    pub names: Vec<&'a str>,
    pub autoplay: Option<&'a str>,
    pub targets: &'a [String],
    /// What each track would key now, in track order.
    pub current: Vec<Option<f32>>,
}

impl EditorApp {
    /// The Timeline panel.
    pub(super) fn timeline_body(&mut self, ui: &mut egui::Ui) {
        let Some(entity) = self.selection.primary() else {
            panel::empty_state(
                ui,
                icons::TIMELINE,
                "Nothing selected",
                "Select an entity to choreograph it and its children: keys that move them over time, and cues scripts can wait for.",
            );
            return;
        };
        let payload = self
            .world
            .get(entity)
            .and_then(|data| data.components.get(SequenceComponent::TYPE_NAME))
            .cloned();
        let Some(payload) = payload else {
            panel::empty_state(
                ui,
                icons::TIMELINE,
                "No sequence here",
                "A sequence moves this entity and its children through keys over time.",
            );
            ui.vertical_centered(|ui| {
                if button::labelled(
                    ui,
                    "Add a sequence",
                    button::Intent::Primary,
                    "Give this entity a Sequence component to choreograph",
                )
                .clicked()
                {
                    self.set_sequence(entity, &default_component(), "Add sequence", false);
                }
            });
            return;
        };
        let component: SequenceComponent = match serde_json::from_value(payload) {
            Ok(component) => component,
            Err(error) => {
                panel::problem(ui, &format!("This sequence cannot be read: {error}"));
                return;
            }
        };
        let frame = ui.ctx().cumulative_frame_nr();
        self.timeline.shown = Some((entity, frame));
        let mut state = std::mem::take(&mut self.timeline);
        let Some((name, sequence)) = state.chosen(&component) else {
            self.timeline = state;
            return;
        };
        if state.playing {
            state.tick(sequence, ui.input(|input| input.stable_dt));
            ui.ctx().request_repaint();
        }
        let every = targets(&self.world, entity);
        let view = View {
            name: &name,
            sequence,
            names: component.sequences.keys().map(String::as_str).collect(),
            autoplay: component.playing.as_deref(),
            targets: &every,
            current: sequence
                .tracks
                .iter()
                .map(|track| current_value(&self.world, entity, track))
                .collect(),
        };
        let (edits, dragging) = timeline_panel(ui, &mut state, &view);
        let structural = edits.iter().any(|edit| {
            matches!(
                edit,
                Edit::AddSequence(_)
                    | Edit::AddTrack { .. }
                    | Edit::RemoveTrack(_)
                    | Edit::RemoveKey { .. }
                    | Edit::AddCue { .. }
                    | Edit::RemoveCue(_)
                    | Edit::Autoplay(_)
                    | Edit::Looping(_)
            )
        });
        if !edits.is_empty() {
            let mut changed = component.clone();
            for edit in edits {
                if let Edit::AddSequence(new) = &edit {
                    state.sequence = Some(new.clone());
                }
                let picked = apply(&mut changed, &name, edit);
                if picked.is_some() {
                    state.picked = picked;
                }
            }
            self.set_sequence(entity, &changed, "Edit sequence", !structural);
        }
        if !dragging && !structural {
            // A drag is one step of history; the next one starts its own.
            if ui.input(|input| input.pointer.any_released()) {
                self.history.break_merge_run();
            }
        }
        self.timeline = state;
    }

    /// Writes the sequence component as one undoable step.
    fn set_sequence(
        &mut self,
        entity: sindri_core::EntityId,
        component: &SequenceComponent,
        label: &str,
        merge: bool,
    ) {
        let Ok(payload) = serde_json::to_value(component) else {
            return;
        };
        let mut buffer = CommandBuffer::new();
        buffer.push(WorldCommand::SetComponent {
            entity,
            type_name: SequenceComponent::TYPE_NAME.to_owned(),
            payload,
        });
        let transaction = buffer.into_transaction(label);
        let transaction = if merge {
            transaction.merging(format!("timeline:{}", entity.index()))
        } else {
            self.history.break_merge_run();
            transaction
        };
        if let Err(error) = self.history.apply(transaction, &mut self.world) {
            self.report(error.to_string());
        }
    }

    /// The world the Scene view draws while the Timeline previews: `source`
    /// with the shown sequence posed at the playhead. `None` when nothing is
    /// previewed, or while Play runs, when the sequence plays for real.
    pub(super) fn timeline_posed(
        &self,
        source: &sindri_core::World,
        frame: u64,
    ) -> Option<sindri_core::World> {
        if !self.timeline.preview || !self.authoring_enabled() {
            return None;
        }
        let (entity, shown) = self.timeline.shown?;
        // Shown this frame or the last: the view may draw before the panel,
        // and a panel no longer drawn poses nothing.
        if shown + 1 < frame {
            return None;
        }
        let component: SequenceComponent = serde_json::from_value(
            source
                .get(entity)?
                .components
                .get(SequenceComponent::TYPE_NAME)?
                .clone(),
        )
        .ok()?;
        let sequence = component
            .sequences
            .get(self.timeline.sequence.as_deref()?)?;
        let mut posed = source.clone();
        sindri_scene::pose(&mut posed, entity, sequence, self.timeline.time);
        Some(posed)
    }
}

fn default_component() -> SequenceComponent {
    SequenceComponent {
        sequences: [(
            "intro".to_owned(),
            Sequence {
                duration: 2.0,
                ..Sequence::default()
            },
        )]
        .into_iter()
        .collect(),
        playing: None,
        speed: 1.0,
    }
}

/// Draws the panel; answers the edits asked for and whether a key or cue is
/// being dragged.
pub(super) fn timeline_panel(
    ui: &mut egui::Ui,
    state: &mut TimelineState,
    view: &View<'_>,
) -> (Vec<Edit>, bool) {
    let mut edits = Vec::new();
    let duration = view.sequence.duration;
    toolbar(ui, state, view, &mut edits);
    panel::rule_tight(ui);
    let mut dragging = false;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let width = ui.available_width();
            let lane = (width - LABEL_WIDTH - metric::GUTTER).max(80.0);
            let top = ui.cursor().top();
            let left = ui.cursor().left() + LABEL_WIDTH;
            let lanes = Lanes {
                left,
                width: lane,
                duration,
            };

            // The ruler: click or drag it to move the playhead.
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(width, RULER), Sense::click_and_drag());
            let ruler = Rect::from_min_max(
                Pos2::new(left, rect.top()),
                Pos2::new(left + lane, rect.bottom()),
            );
            ruler_marks(ui, ruler, duration);
            if (response.dragged() || response.clicked())
                && let Some(at) = response.interact_pointer_pos()
            {
                state.time = lanes.time_at(at.x);
                state.playing = false;
            }
            dragging |= cue_lane(ui, &lanes, width, state, view, &mut edits);
            for index in 0..view.sequence.tracks.len() {
                dragging |= track_lane(ui, &lanes, width, index, state, view, &mut edits);
            }
            if view.sequence.tracks.is_empty() {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(metric::GUTTER);
                    ui.label(
                        RichText::new("No tracks yet: add one below, then key it at the playhead.")
                            .size(text::NOTE)
                            .color(color::TEXT_FAINT),
                    );
                });
            }

            // The playhead, over everything above.
            let bottom = ui.cursor().top();
            let x = lanes.x_of(state.time);
            ui.painter()
                .vline(x, top..=bottom, Stroke::new(1.5, color::FORGE));
            ui.painter()
                .circle_filled(Pos2::new(x, top + 4.0), 4.0, color::FORGE);

            ui.spacing_mut().item_spacing.y = 4.0;
            ui.add_space(8.0);
            panel::body(ui, |ui| {
                add_track(ui, state, view, &mut edits);
                ui.add_space(metric::GROUP_GAP);
                picked_editor(ui, state, view, &mut edits);
            });
        });
    (edits, dragging)
}

/// Where a time sits across the lanes, and the time under a point.
struct Lanes {
    left: f32,
    width: f32,
    duration: f32,
}

impl Lanes {
    fn x_of(&self, time: f32) -> f32 {
        self.left + self.width * (time / self.duration.max(1e-3)).clamp(0.0, 1.0)
    }

    fn time_at(&self, x: f32) -> f32 {
        snapped((x - self.left) / self.width * self.duration, self.duration)
    }
}

/// The cue lane; answers whether a cue is being dragged.
fn cue_lane(
    ui: &mut egui::Ui,
    lanes: &Lanes,
    width: f32,
    state: &mut TimelineState,
    view: &View<'_>,
    edits: &mut Vec<Edit>,
) -> bool {
    let mut dragging = false;
    let cues = lane_row(ui, width, "Cues", None);
    if icon_at(
        ui,
        cues.button,
        glyphs::ICON_ADD,
        "Add a cue at the playhead",
    ) {
        edits.push(Edit::AddCue { time: state.time });
    }
    for (index, cue) in view.sequence.cues.iter().enumerate() {
        let picked = state.picked == Some(Picked::Cue(index));
        let at = Pos2::new(lanes.x_of(cue.time), cues.lane.center().y);
        let handle = ui.interact(
            Rect::from_center_size(at, Vec2::splat(14.0)),
            ui.id().with(("cue", index)),
            Sense::click_and_drag(),
        );
        if handle.clicked() || handle.drag_started() {
            state.picked = Some(Picked::Cue(index));
        }
        if handle.dragged()
            && let Some(pointer) = handle.interact_pointer_pos()
        {
            dragging = true;
            edits.push(Edit::SetCue {
                cue: index,
                time: lanes.time_at(pointer.x),
                name: cue.name.clone(),
                sound: cue
                    .sound
                    .as_ref()
                    .map(|s| s.clip.clone())
                    .unwrap_or_default(),
            });
        }
        flag(ui, at, picked, cue.sound.is_some());
        handle.on_hover_text(format!("{} at {:.2} s", cue.name, cue.time));
    }
    dragging
}

/// One track's lane; answers whether one of its keys is being dragged.
fn track_lane(
    ui: &mut egui::Ui,
    lanes: &Lanes,
    width: f32,
    index: usize,
    state: &mut TimelineState,
    view: &View<'_>,
    edits: &mut Vec<Edit>,
) -> bool {
    let mut dragging = false;
    let track = &view.sequence.tracks[index];
    let label = if track.target.is_empty() {
        short_property(&track.property)
    } else {
        format!("{} · {}", track.target, short_property(&track.property))
    };
    let row = lane_row(ui, width, &label, Some(index));
    if icon_at(
        ui,
        row.button,
        glyphs::ICON_KEY,
        "Key the value the scene holds now, at the playhead",
    ) && let Some(value) = view.current.get(index).copied().flatten()
    {
        edits.push(Edit::Key {
            track: index,
            time: state.time,
            value,
        });
    }
    if icon_at(ui, row.remove, glyphs::ICON_CLOSE, "Remove this track") {
        edits.push(Edit::RemoveTrack(index));
    }
    if let [first, .., last] = track.keys.as_slice() {
        ui.painter().hline(
            lanes.x_of(first.time)..=lanes.x_of(last.time),
            row.lane.center().y,
            Stroke::new(1.0, color::LINE),
        );
    }
    for (key_index, key) in track.keys.iter().enumerate() {
        let picked = state.picked
            == Some(Picked::Key {
                track: index,
                key: key_index,
            });
        let at = Pos2::new(lanes.x_of(key.time), row.lane.center().y);
        let handle = ui.interact(
            Rect::from_center_size(at, Vec2::splat(14.0)),
            ui.id().with(("key", index, key_index)),
            Sense::click_and_drag(),
        );
        if handle.clicked() || handle.drag_started() {
            state.picked = Some(Picked::Key {
                track: index,
                key: key_index,
            });
        }
        if handle.dragged()
            && let Some(pointer) = handle.interact_pointer_pos()
        {
            dragging = true;
            let time = lanes.time_at(pointer.x);
            if (time - key.time).abs() > f32::EPSILON {
                edits.push(Edit::MoveKey {
                    track: index,
                    key: key_index,
                    time,
                });
            }
        }
        diamond(ui, at, picked);
        handle.on_hover_text(format!(
            "{:.2} at {:.2} s ({})",
            key.value, key.time, key.ease
        ));
    }
    dragging
}

/// A property as a row can fit it: a component field loses the component's
/// namespace, so `sindri.shape/stroke.3` reads `shape stroke.3`.
fn short_property(property: &str) -> String {
    match property.split_once('/') {
        Some((component, path)) => {
            let short = component.rsplit('.').next().unwrap_or(component);
            format!("{short} {path}")
        }
        None => property.to_owned(),
    }
}

fn toolbar(ui: &mut egui::Ui, state: &mut TimelineState, view: &View<'_>, edits: &mut Vec<Edit>) {
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

/// A row's parts: the lane keys sit in, and its label's two buttons.
struct Row {
    lane: Rect,
    button: Rect,
    remove: Rect,
}

fn lane_row(ui: &mut egui::Ui, width: f32, label: &str, track: Option<usize>) -> Row {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, ROW), Sense::hover());
    let painter = ui.painter();
    let stripe = track.is_some_and(|track| track % 2 == 1);
    if stripe {
        painter.rect_filled(rect, 0.0, color::WELL.gamma_multiply(0.5));
    }
    painter.hline(
        rect.x_range(),
        rect.bottom(),
        Stroke::new(1.0, color::LINE_SOFT),
    );
    let name = Rect::from_min_max(
        Pos2::new(rect.left() + metric::GUTTER, rect.top()),
        Pos2::new(rect.left() + LABEL_WIDTH - 44.0, rect.bottom()),
    );
    let galley = painter.layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(text::LABEL),
        if track.is_some() {
            color::TEXT
        } else {
            color::TEXT_MUTED
        },
    );
    painter.with_clip_rect(name).galley(
        Pos2::new(name.left(), name.center().y - galley.size().y / 2.0),
        galley,
        Color32::WHITE,
    );
    let button = Rect::from_center_size(
        Pos2::new(rect.left() + LABEL_WIDTH - 32.0, rect.center().y),
        Vec2::splat(18.0),
    );
    let remove = if track.is_some() {
        Rect::from_center_size(
            Pos2::new(rect.left() + LABEL_WIDTH - 12.0, rect.center().y),
            Vec2::splat(18.0),
        )
    } else {
        Rect::NOTHING
    };
    Row {
        lane: Rect::from_min_max(Pos2::new(rect.left() + LABEL_WIDTH, rect.top()), rect.max),
        button,
        remove,
    }
}

fn ruler_marks(ui: &egui::Ui, ruler: Rect, duration: f32) {
    let painter = ui.painter();
    painter.rect_filled(ruler, 0.0, color::WELL);
    let step = if duration <= 3.0 {
        0.25
    } else if duration <= 12.0 {
        1.0
    } else {
        5.0
    };
    let mut time = 0.0_f32;
    while time <= duration + 1e-4 {
        let x = ruler.left() + ruler.width() * time / duration.max(1e-3);
        let whole = (time / (step * 2.0)).fract().abs() < 1e-3;
        painter.vline(
            x,
            if whole {
                ruler.top() + 6.0
            } else {
                ruler.top() + 12.0
            }..=ruler.bottom(),
            Stroke::new(1.0, color::TEXT_FAINT),
        );
        if whole {
            painter.text(
                Pos2::new(x + 3.0, ruler.top() + 2.0),
                egui::Align2::LEFT_TOP,
                format!("{time:.2}")
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_owned()
                    + "s",
                egui::FontId::proportional(text::NOTE),
                color::TEXT_MUTED,
            );
        }
        time += step;
    }
}

/// An icon button placed in a rectangle a row worked out, rather than laid
/// out after the last widget.
fn icon_at(
    ui: &mut egui::Ui,
    rect: Rect,
    glyph: egui_material_icons::MaterialIcon,
    tip: &str,
) -> bool {
    if rect == Rect::NOTHING {
        return false;
    }
    ui.put(
        rect,
        egui::Button::new(
            glyph
                .outlined()
                .rich_text()
                .size(14.0)
                .color(color::TEXT_MUTED),
        )
        .frame(false),
    )
    .on_hover_text(tip)
    .clicked()
}

fn diamond(ui: &egui::Ui, at: Pos2, picked: bool) {
    let r = if picked { 6.0 } else { 5.0 };
    let points = vec![
        Pos2::new(at.x, at.y - r),
        Pos2::new(at.x + r, at.y),
        Pos2::new(at.x, at.y + r),
        Pos2::new(at.x - r, at.y),
    ];
    ui.painter().add(egui::Shape::convex_polygon(
        points,
        if picked {
            color::FORGE
        } else {
            color::TEXT_MUTED
        },
        Stroke::new(1.0, if picked { color::TEXT } else { color::LINE }),
    ));
}

fn flag(ui: &egui::Ui, at: Pos2, picked: bool, sounds: bool) {
    let fill = if picked {
        color::FORGE
    } else if sounds {
        color::AXIS_Y
    } else {
        color::AXIS_Z
    };
    let painter = ui.painter();
    painter.vline(at.x, (at.y - 8.0)..=(at.y + 8.0), Stroke::new(1.5, fill));
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(at.x, at.y - 8.0),
            Pos2::new(at.x + 8.0, at.y - 5.0),
            Pos2::new(at.x, at.y - 2.0),
        ],
        fill,
        Stroke::NONE,
    ));
}

/// Choosing what a new track moves.
fn add_track(ui: &mut egui::Ui, state: &mut TimelineState, view: &View<'_>, edits: &mut Vec<Edit>) {
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
fn picked_editor(
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
fn key_editor(
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
fn cue_editor(
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

#[cfg(test)]
mod tests;
