//! The Timeline panel: the selected entity's sequences as rows of keys
//! against time, with a playhead that poses the Scene view.

use eframe::egui::{self, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use sindri_core::{CommandBuffer, SceneComponent, WorldCommand};
use sindri_scene::{Sequence, SequenceComponent};

use crate::timeline::{Edit, TimelineState, apply, current_value, targets};
use crate::ui::icons;
use crate::ui::theme::{color, metric, text};
use crate::ui::widgets::{button, panel};

use super::EditorApp;

const LABEL_WIDTH: f32 = 170.0;
const ROW: f32 = 24.0;
const RULER: f32 = 22.0;

mod editors;
mod lanes;

use editors::{add_track, picked_editor, toolbar};
use lanes::{Lanes, cue_lane, ruler_marks, track_lane};

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

#[cfg(test)]
mod tests;
