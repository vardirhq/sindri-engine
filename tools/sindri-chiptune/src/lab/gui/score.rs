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
    (base, begin): (i32, usize),
    hover: Option<egui::Pos2>,
) {
    let row = grid.height() / 24.0;
    let step = grid.width() / 128.0;
    for pitch in base..base + 24 {
        let y = grid.bottom() - f32::from(i16::try_from(pitch - base + 1).expect("pitch")) * row;
        let black = [1, 3, 6, 8, 10].contains(&pitch.rem_euclid(12));
        let band =
            egui::Rect::from_min_size(egui::pos2(grid.left(), y), egui::vec2(grid.width(), row));
        painter.rect_filled(
            band,
            0.0,
            if black {
                Color32::from_rgb(13, 16, 20)
            } else {
                Color32::from_rgb(18, 22, 27)
            },
        );
        // The keyboard gutter: white and black keys, with every C named.
        let key = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 2.0, y),
            egui::vec2(KEYS_WIDTH - 4.0, row),
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
    for tick in 0..=128 {
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
        if tick < 128 && tick % 16 == 0 {
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

impl ComposerApp {
    pub(super) fn score(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        self.overview(ui);
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            for (track, title) in ["Bass", "Arpeggio", "Lead"].iter().enumerate() {
                let color = TRACK_COLORS[track];
                let on = self.edit_track == track;
                let text = RichText::new(*title)
                    .color(if on { color } else { theme::MUTED })
                    .strong();
                let tab = egui::Button::new(text)
                    .fill(if on {
                        color.gamma_multiply(0.14)
                    } else {
                        theme::WELL
                    })
                    .stroke(egui::Stroke::new(
                        1.0,
                        if on { color } else { theme::LINE_SOFT },
                    ))
                    .min_size(egui::vec2(84.0, 26.0));
                if ui.add(tab).clicked() {
                    self.edit_track = track;
                    self.fit_pitch_range();
                    self.selected_note = None;
                }
            }
            ui.add_space(12.0);
            ui.label(RichText::new("New note").color(theme::FAINT));
            ui.add(
                egui::DragValue::new(&mut self.note_length)
                    .range(1..=16)
                    .suffix(" steps"),
            );
            ui.label(RichText::new("Window").color(theme::FAINT));
            ui.add(
                egui::DragValue::new(&mut self.roll_base)
                    .range(0..=104)
                    .prefix("from "),
            );
            if ui
                .button("Fit")
                .on_hover_text("Fit the window to this voice's notes")
                .clicked()
            {
                self.fit_pitch_range();
            }
        });
        self.note_inspector(ui);
        egui::ScrollArea::horizontal()
            .id_salt("piano-roll")
            .show(ui, |ui| {
                self.piano_roll(ui);
            });
        ui.label(
            RichText::new(if self.compact {
                "Tap to add · drag to move · long-press to delete · tap the ruler to seek"
            } else {
                "Click to add · drag to move · right-click to delete · click the ruler to seek · snap 1/16"
            })
            .size(11.0)
            .color(theme::FAINT),
        );
    }

    pub(super) fn fit_pitch_range(&mut self) {
        let low = self.scores[self.selected][self.edit_track]
            .iter()
            .map(|note| note.pitch)
            .min()
            .unwrap_or(48);
        self.roll_base = (low - 3).clamp(0, 104);
    }

    fn note_inspector(&mut self, ui: &mut egui::Ui) {
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
        let width = ui.available_width().max(720.0);
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(width, if self.compact { 380.0 } else { 480.0 }),
            egui::Sense::click_and_drag(),
        );
        let painter = ui.painter_at(rect);
        let grid = egui::Rect::from_min_max(rect.min + egui::vec2(KEYS_WIDTH, 24.0), rect.max);
        let base = self.roll_base;
        let row = grid.height() / 24.0;
        let step = grid.width() / 128.0;
        let begin = self.section * 128;
        theme::well(&painter, rect);
        let hover = response.hover_pos().filter(|pos| grid.contains(*pos));
        lanes(&painter, rect, grid, (base, begin), hover);
        let place = |note: &Note| -> Option<egui::Rect> {
            if note.start + note.len <= begin
                || note.start >= begin + 128
                || !(base..base + 24).contains(&note.pitch)
            {
                return None;
            }
            let x = grid.left() + display(note.start.saturating_sub(begin)) * step;
            let w =
                display((note.start + note.len).min(begin + 128) - note.start.max(begin)) * step;
            let y = grid.bottom()
                - f32::from(i16::try_from(note.pitch - base + 1).expect("pitch")) * row;
            Some(egui::Rect::from_min_size(
                egui::pos2(x + 1.0, y + 1.5),
                egui::vec2((w - 2.0).max(3.0), row - 3.0),
            ))
        };
        // The other two voices, faint, so a new note can be placed against them.
        for (track, notes) in self.scores[self.selected].iter().enumerate() {
            if track == self.edit_track {
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
        let color = TRACK_COLORS[self.edit_track];
        for (index, note) in self.scores[self.selected][self.edit_track]
            .iter()
            .enumerate()
        {
            let Some(r) = place(note) else { continue };
            let selected = self.selected_note == Some(index);
            let level = 0.4 + 0.6 * f32::from(note.velocity) / 127.0;
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
        let playhead = self.playhead_bar() * 16.0 - display(begin);
        if self.transport.is_some() && (0.0..128.0).contains(&playhead) {
            let x = grid.left() + playhead * step;
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(2.0, ACCENT),
            );
            painter.circle_filled(egui::pos2(x, rect.top() + 4.0), 4.0, ACCENT);
        }
        self.roll_interaction(&response, grid, base, step, row, begin);
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
        let slot = ((pos.x - grid.left()) / step).floor().clamp(0.0, 127.0) as usize + begin;
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
        let pitch = base + ((grid.bottom() - pos.y) / row).floor().clamp(0.0, 23.0) as i32;
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
