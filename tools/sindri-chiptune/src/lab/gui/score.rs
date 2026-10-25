//! Editable note-event piano roll; every edit is reflected in audition and export.
use super::{ACCENT, ComposerApp, TRACK_COLORS, theme};
use crate::lab::{Note, STEPS, live::Command};
use eframe::egui::{self, Color32, RichText};

const KEYS_WIDTH: f32 = 46.0;

fn note_name(pitch: i32) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let index = usize::try_from(pitch.rem_euclid(12)).expect("pitch class");
    format!("{}{}", NAMES[index], pitch.div_euclid(12) - 1)
}

/// The keyboard gutter, pitch rows, bar lines and ruler numbers.
fn lanes(
    painter: &egui::Painter,
    rect: egui::Rect,
    grid: egui::Rect,
    (base, begin, rows, steps): (i32, usize, usize, usize),
    hover: Option<egui::Pos2>,
) {
    let row = grid.height() / display(rows);
    let step = grid.width() / display(steps);
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(rect.left(), grid.top()),
            egui::pos2(grid.left(), grid.bottom()),
        ),
        0.0,
        Color32::from_rgb(196, 202, 209),
    );
    for pitch in base..base + i32::try_from(rows).expect("pitch rows") {
        let y = grid.bottom() - f32::from(i16::try_from(pitch - base + 1).expect("pitch")) * row;
        let black = [1, 3, 6, 8, 10].contains(&pitch.rem_euclid(12));
        let band =
            egui::Rect::from_min_size(egui::pos2(grid.left(), y), egui::vec2(grid.width(), row));
        painter.rect_filled(
            band,
            0.0,
            if black {
                Color32::from_rgb(8, 22, 32)
            } else {
                Color32::from_rgb(13, 29, 42)
            },
        );
        // The keyboard gutter: white and black keys, with every C named.
        let key = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 2.0, y),
            egui::vec2(
                if black {
                    KEYS_WIDTH * 0.63
                } else {
                    KEYS_WIDTH - 4.0
                },
                row,
            ),
        );
        let lit = hover.is_some_and(|pos| band.y_range().contains(pos.y));
        let key_fill = match (black, lit) {
            (_, true) => theme::FORGE_DIM,
            (true, false) => Color32::from_rgb(30, 34, 40),
            (false, false) => Color32::from_rgb(196, 192, 184),
        };
        painter.rect_filled(key.shrink2(egui::vec2(0.0, 0.5)), 2.0, key_fill);
        if pitch % 12 == 0 {
            painter.text(
                egui::pos2(key.right() - 4.0, key.center().y),
                egui::Align2::RIGHT_CENTER,
                format!("C{}", pitch / 12 - 1),
                egui::FontId::monospace(9.5),
                Color32::from_rgb(40, 44, 50),
            );
            painter.line_segment(
                [
                    egui::pos2(grid.left(), y + row),
                    egui::pos2(grid.right(), y + row),
                ],
                egui::Stroke::new(1.0, theme::LINE),
            );
        }
    }
    for tick in 0..=steps {
        let x = grid.left() + display(tick) * step;
        let (weight, shade) = match tick {
            t if t % 16 == 0 => (1.0, theme::LINE),
            t if t % 4 == 0 => (1.0, theme::LINE_SOFT),
            _ => (0.5, Color32::from_rgb(22, 26, 31)),
        };
        painter.line_segment(
            [egui::pos2(x, grid.top()), egui::pos2(x, grid.bottom())],
            egui::Stroke::new(weight, shade),
        );
        if tick < steps && tick % 16 == 0 {
            painter.text(
                egui::pos2(x + 5.0, rect.top() + 5.0),
                egui::Align2::LEFT_TOP,
                format!("{:02}", (begin + tick) / 16 + 1),
                egui::FontId::monospace(11.0),
                theme::MUTED,
            );
        }
    }
}

fn display(value: usize) -> f32 {
    f32::from(u16::try_from(value).expect("bounded score coordinate"))
}

fn paint_hover(
    painter: &egui::Painter,
    grid: egui::Rect,
    hover: Option<egui::Pos2>,
    step: f32,
    row: f32,
    color: Color32,
) {
    if let Some(pos) = hover {
        let column = ((pos.x - grid.left()) / step).floor();
        let cell = egui::Rect::from_min_size(
            egui::pos2(
                grid.left() + column * step,
                grid.bottom() - ((grid.bottom() - pos.y) / row).ceil() * row,
            ),
            egui::vec2(step, row),
        );
        painter.rect_stroke(
            cell,
            1.0,
            egui::Stroke::new(1.0, color.gamma_multiply(0.6)),
            egui::StrokeKind::Inside,
        );
    }
}

impl ComposerApp {
    pub(super) fn score(&mut self, ui: &mut egui::Ui) {
        self.overview(ui);
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(super::overview::LAYERS[self.selected_layer])
                    .color(TRACK_COLORS[self.selected_layer])
                    .strong(),
            );
            if self.selected_layer < 3 {
                ui.add(
                    egui::DragValue::new(&mut self.note_length)
                        .range(1..=16)
                        .suffix(" steps"),
                );
            } else {
                ui.label(RichText::new("Generated").color(theme::MUTED));
            }
            if !self.compact {
                ui.add(
                    egui::DragValue::new(&mut self.roll_base)
                        .range(0..=104)
                        .prefix("from "),
                );
            }
            if ui
                .button("Fit")
                .on_hover_text("Fit the window to this voice's notes")
                .clicked()
            {
                self.fit_pitch_range();
            }
            if self.compact {
                if ui.small_button("1–4").clicked() {
                    self.roll_page = 0;
                    self.fit_pitch_range();
                }
                if ui.small_button("5–8").clicked() {
                    self.roll_page = 1;
                    self.fit_pitch_range();
                }
            }
        });
        self.note_inspector(ui);
        egui::ScrollArea::horizontal()
            .id_salt("piano-roll")
            .show(ui, |ui| {
                self.piano_roll(ui);
            });
        ui.label(
            RichText::new(if self.selected_layer >= 3 {
                "Generated layer preview · select this layer to change its sound"
            } else if self.compact {
                "Tap to add · drag to move · long-press to delete · tap the ruler to seek"
            } else {
                "Click to add · drag to move · right-click to delete · click the ruler to seek · snap 1/16"
            })
            .size(11.0)
            .color(theme::FAINT),
        );
    }

    /// Prefer a populated phone page on selection; explicit page buttons still allow empty bars.
    pub(super) fn focus_score_window(&mut self) {
        self.refresh_arrangement();
        let begin = self.section * 128 + self.roll_page * 64;
        if self.compact
            && !self
                .layer_notes(self.selected_layer)
                .iter()
                .any(|n| n.start < begin + 64 && n.start + n.len > begin)
        {
            let section_begin = self.section * 128;
            if let Some(first) = self
                .layer_notes(self.selected_layer)
                .iter()
                .find(|n| n.start < section_begin + 128 && n.start + n.len > section_begin)
            {
                self.roll_page = first.start.saturating_sub(section_begin) / 64;
            }
        }
        self.fit_pitch_range();
    }

    pub(super) fn fit_pitch_range(&mut self) {
        let begin = self.section * 128 + if self.compact { self.roll_page * 64 } else { 0 };
        let steps = if self.compact { 64 } else { 128 };
        let notes = self.layer_notes(self.selected_layer);
        let visible: Vec<_> = notes
            .iter()
            .filter(|n| n.start < begin + steps && n.start + n.len > begin)
            .collect();
        let low = visible.iter().map(|n| n.pitch).min().unwrap_or(60);
        let high = visible.iter().map(|n| n.pitch).max().unwrap_or(72);
        self.roll_base = (low - 2).clamp(0, 104);
        self.roll_span =
            usize::try_from((high - self.roll_base + 3).clamp(12, 128 - self.roll_base))
                .expect("pitch range");
    }

    fn note_inspector(&mut self, ui: &mut egui::Ui) {
        if self.selected_layer >= 3 {
            return;
        }
        let Some(index) = self.selected_note else {
            return;
        };
        let Some(mut note) = self.scores[self.selected][self.edit_track]
            .get(index)
            .copied()
        else {
            self.selected_note = None;
            return;
        };
        let before = note;
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("SELECTED")
                    .size(10.5)
                    .strong()
                    .color(TRACK_COLORS[self.edit_track]),
            );
            ui.label(
                RichText::new(note_name(note.pitch))
                    .monospace()
                    .color(theme::TEXT),
            );
            ui.add(
                egui::DragValue::new(&mut note.pitch)
                    .range(0..=127)
                    .prefix("Pitch "),
            );
            ui.add(
                egui::DragValue::new(&mut note.len)
                    .range(1..=STEPS - note.start)
                    .prefix("Length "),
            );
            ui.add(
                egui::DragValue::new(&mut note.velocity)
                    .range(1..=127)
                    .prefix("Velocity "),
            );
        });
        if note != before {
            self.save_undo();
            self.scores[self.selected][self.edit_track][index] = note;
            self.finish_edit();
        }
    }

    fn piano_roll(&mut self, ui: &mut egui::Ui) {
        let width = ui.available_width();
        let rows = self.roll_span;
        let steps = if self.compact { 64 } else { 128 };
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(width, if self.compact { 172.0 } else { 380.0 }),
            egui::Sense::click_and_drag(),
        );
        let painter = ui.painter_at(rect);
        let grid = egui::Rect::from_min_max(
            rect.min + egui::vec2(KEYS_WIDTH, 24.0),
            rect.max - egui::vec2(0.0, 30.0),
        );
        let base = self.roll_base;
        let row = grid.height() / display(rows);
        let step = grid.width() / display(steps);
        let begin = self.section * 128 + if self.compact { self.roll_page * 64 } else { 0 };
        theme::well(&painter, rect);
        let hover = response.hover_pos().filter(|pos| grid.contains(*pos));
        lanes(&painter, rect, grid, (base, begin, rows, steps), hover);
        let place = |note: &Note| -> Option<egui::Rect> {
            if note.start + note.len <= begin
                || note.start >= begin + steps
                || !(base..base + i32::try_from(rows).expect("pitch rows")).contains(&note.pitch)
            {
                return None;
            }
            let x = grid.left() + display(note.start.saturating_sub(begin)) * step;
            let w =
                display((note.start + note.len).min(begin + steps) - note.start.max(begin)) * step;
            let y = grid.bottom()
                - f32::from(i16::try_from(note.pitch - base + 1).expect("pitch")) * row;
            Some(egui::Rect::from_min_size(
                egui::pos2(x + 1.0, y + 1.5),
                egui::vec2((w - 2.0).max(3.0), row - 3.0),
            ))
        };
        // The other two voices, faint, so a new note can be placed against them.
        for (track, notes) in self.scores[self.selected].iter().enumerate() {
            if self.selected_layer >= 3 || track == self.edit_track {
                continue;
            }
            for note in notes {
                if let Some(r) = place(note) {
                    painter.rect_stroke(
                        r,
                        2.0,
                        egui::Stroke::new(1.0, TRACK_COLORS[track].gamma_multiply(0.3)),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        let color = TRACK_COLORS[self.selected_layer];
        for (index, note) in self.layer_notes(self.selected_layer).iter().enumerate() {
            let Some(r) = place(note) else { continue };
            let selected = self.selected_note == Some(index);
            let level = 0.85 + 0.15 * f32::from(note.velocity) / 127.0;
            if selected {
                painter.rect_filled(r.expand(3.0), 4.0, color.gamma_multiply(0.25));
            }
            painter.rect_filled(r, 2.5, color.gamma_multiply(level));
            painter.rect_filled(
                egui::Rect::from_min_size(r.min, egui::vec2(r.width(), 2.0)),
                1.0,
                Color32::WHITE.gamma_multiply(0.35),
            );
            if selected {
                painter.rect_stroke(
                    r,
                    2.5,
                    egui::Stroke::new(1.5, Color32::WHITE),
                    egui::StrokeKind::Outside,
                );
            }
        }
        self.velocity_lane(&painter, rect, grid, (begin, steps));
        paint_hover(&painter, grid, hover, step, row, color);
        let playhead = self.playhead_bar() * 16.0 - display(begin);
        if self.transport.is_some() && (0.0..display(steps)).contains(&playhead) {
            let x = grid.left() + playhead * step;
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(2.0, ACCENT),
            );
            painter.circle_filled(egui::pos2(x, rect.top() + 4.0), 4.0, ACCENT);
        }
        if self.selected_layer < 3 {
            self.roll_interaction(&response, grid, base, step, row, begin);
        }
    }

    fn velocity_lane(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        grid: egui::Rect,
        (begin, steps): (usize, usize),
    ) {
        let step = grid.width() / display(steps);
        let color = TRACK_COLORS[self.selected_layer];
        painter.text(
            egui::pos2(rect.left() + 4.0, grid.bottom() + 15.0),
            egui::Align2::LEFT_CENTER,
            "Vel",
            egui::FontId::proportional(10.0),
            theme::MUTED,
        );
        for note in self.layer_notes(self.selected_layer) {
            if note.start >= begin && note.start < begin + steps {
                let x = grid.left() + display(note.start - begin) * step;
                let height = 23.0 * f32::from(note.velocity) / 127.0;
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(x, rect.bottom() - height - 2.0),
                        egui::vec2(2.0, height),
                    ),
                    0.0,
                    color,
                );
            }
        }
    }

    // Pointer coordinates are clamped to the finite piano-roll grid before snapping.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn roll_interaction(
        &mut self,
        response: &egui::Response,
        grid: egui::Rect,
        base: i32,
        step: f32,
        row: f32,
        begin: usize,
    ) {
        if response.drag_stopped() && self.drag_origin.take().is_some() {
            self.finish_edit();
        }
        let Some(pos) = response.interact_pointer_pos() else {
            return;
        };
        let pos = if response.drag_started() {
            pos - response.total_drag_delta().unwrap_or_default()
        } else {
            pos
        };
        let slot = ((pos.x - grid.left()) / step)
            .floor()
            .clamp(0.0, (grid.width() / step - 1.0).max(0.0)) as usize
            + begin;
        if pos.y < grid.top() {
            if response.clicked()
                && let Some(transport) = &self.transport
            {
                let _ = transport.tx.send(Command::Seek(slot / 16));
            }
            return;
        }
        if !grid.contains(pos) {
            return;
        }
        let pitch = base
            + ((grid.bottom() - pos.y) / row)
                .floor()
                .clamp(0.0, (grid.height() / row - 1.0).max(0.0)) as i32;
        let hit = self.scores[self.selected][self.edit_track]
            .iter()
            .position(|n| n.pitch == pitch && (n.start..n.start + n.len).contains(&slot));
        if response.drag_started() {
            self.selected_note = hit;
            if let Some(index) = hit {
                self.save_undo();
                self.drag_origin = Some((
                    index,
                    self.scores[self.selected][self.edit_track][index],
                    slot,
                    pitch,
                ));
            }
        }
        if response.dragged()
            && let Some((index, original, first_slot, first_pitch)) = self.drag_origin
        {
            let note = &mut self.scores[self.selected][self.edit_track][index];
            let delta =
                i32::try_from(slot).expect("slot") - i32::try_from(first_slot).expect("slot");
            note.start = usize::try_from(
                (i32::try_from(original.start).expect("start") + delta)
                    .clamp(0, i32::try_from(STEPS - original.len).expect("end")),
            )
            .expect("start");
            note.pitch = (original.pitch + pitch - first_pitch).clamp(0, 127);
        }
        if response.secondary_clicked() {
            if let Some(index) = hit {
                self.save_undo();
                self.scores[self.selected][self.edit_track].remove(index);
                self.selected_note = None;
                self.finish_edit();
            }
        } else if response.clicked() {
            self.selected_note = hit;
            if hit.is_none() {
                self.save_undo();
                self.selected_note = Some(self.scores[self.selected][self.edit_track].len());
                self.scores[self.selected][self.edit_track].push(Note {
                    pitch,
                    start: slot,
                    len: self.note_length.min(STEPS - slot),
                    velocity: 88,
                });
                self.finish_edit();
            }
        }
    }
}
