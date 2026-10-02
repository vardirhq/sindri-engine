//! The lanes a timeline is drawn in: the ruler's marks, a row per track or
//! the cues, and the keys and flags that sit on them.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use egui_material_icons::icons as glyphs;

use crate::timeline::{Edit, Picked, TimelineState, snapped};
use crate::ui::theme::{color, metric, text};

use super::{LABEL_WIDTH, ROW, View};

/// Where a time sits across the lanes, and the time under a point.
pub(super) struct Lanes {
    pub(super) left: f32,
    pub(super) width: f32,
    pub(super) duration: f32,
}

impl Lanes {
    pub(super) fn x_of(&self, time: f32) -> f32 {
        self.left + self.width * (time / self.duration.max(1e-3)).clamp(0.0, 1.0)
    }

    pub(super) fn time_at(&self, x: f32) -> f32 {
        snapped((x - self.left) / self.width * self.duration, self.duration)
    }
}

/// The cue lane; answers whether a cue is being dragged.
pub(super) fn cue_lane(
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
pub(super) fn track_lane(
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
pub(super) fn short_property(property: &str) -> String {
    match property.split_once('/') {
        Some((component, path)) => {
            let short = component.rsplit('.').next().unwrap_or(component);
            format!("{short} {path}")
        }
        None => property.to_owned(),
    }
}

/// A row's parts: the lane keys sit in, and its label's two buttons.
pub(super) struct Row {
    lane: Rect,
    button: Rect,
    remove: Rect,
}

pub(super) fn lane_row(ui: &mut egui::Ui, width: f32, label: &str, track: Option<usize>) -> Row {
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

pub(super) fn ruler_marks(ui: &egui::Ui, ruler: Rect, duration: f32) {
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
pub(super) fn icon_at(
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

pub(super) fn diamond(ui: &egui::Ui, at: Pos2, picked: bool) {
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

pub(super) fn flag(ui: &egui::Ui, at: Pos2, picked: bool, sounds: bool) {
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
