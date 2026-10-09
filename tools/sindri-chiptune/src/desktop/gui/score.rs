//! Editable note-event piano roll; every edit is reflected in audition and export.
use super::{ACCENT, ComposerApp, TRACK_COLORS};
use crate::desktop::{Note, STEPS, live::Command};
use eframe::egui::{self, Color32, RichText};

fn display(value: usize) -> f32 {
    f32::from(u16::try_from(value).expect("bounded score coordinate"))
}

impl ComposerApp {
    pub(super) fn score(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.heading("Chiptune workstation");
            ui.label(RichText::new("32 bars / 4 voices").color(ACCENT));
        });
        ui.horizontal_wrapped(|ui| {
            for candidate in 0..2 {
                let label = format!(
                    "{} · seed {}",
                    if candidate == 0 { "A" } else { "B" },
                    self.generated
                        .seed
                        .wrapping_add(u64::try_from(candidate).expect("candidate"))
                );
                if ui
                    .selectable_label(self.selected == candidate, label)
                    .clicked()
                {
                    self.selected = candidate;
                    self.fit_pitch_range();
                    self.selected_note = None;
                    if self.transport.is_some() {
                        self.start_live();
                    }
                }
            }
            if ui
                .add_enabled(!self.undo.is_empty(), egui::Button::new("Undo"))
                .clicked()
            {
                self.undo_edit();
            }
            if ui
                .add_enabled(!self.redo.is_empty(), egui::Button::new("Redo"))
                .clicked()
            {
                self.redo_edit();
            }
        });
        ui.add_space(8.0);
        ui.columns(4, |columns| {
            for (section, title) in ["01 / INTRO", "02 / THEME", "03 / RISE", "04 / RETURN"]
                .iter()
                .enumerate()
            {
                if columns[section]
                    .selectable_label(self.section == section, *title)
                    .clicked()
                {
                    self.section = section;
                }
                columns[section].small(format!("Bars {}–{}", section * 8 + 1, section * 8 + 8));
            }
        });
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            for (track, title) in ["Bass", "Arpeggio", "Lead"].iter().enumerate() {
                if ui
                    .selectable_label(
                        self.edit_track == track,
                        RichText::new(*title).color(TRACK_COLORS[track]),
                    )
                    .clicked()
                {
                    self.edit_track = track;
                    self.fit_pitch_range();
                    self.selected_note = None;
                }
            }
            ui.label("New note");
            ui.add(
                egui::DragValue::new(&mut self.note_length)
                    .range(1..=16)
                    .suffix(" steps"),
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Pitch window");
            ui.add(egui::DragValue::new(&mut self.roll_base).range(0..=104));
            if ui.button("Fit notes").clicked() {
                self.fit_pitch_range();
            }
        });
        ui.small("Click to add/select · drag to move · right-click to delete · snap: 1/16");
        self.note_inspector(ui);
        egui::ScrollArea::both()
            .id_salt("piano-roll")
            .max_height(510.0)
            .show(ui, |ui| {
                self.piano_roll(ui);
            });
        ui.small("Click the bar ruler to seek. Colored line follows live playback.");
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
            ui.label("Selected note");
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
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(1200.0, 504.0), egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        let grid = egui::Rect::from_min_max(rect.min + egui::vec2(48.0, 24.0), rect.max);
        let base = self.roll_base;
        let row = grid.height() / 24.0;
        let step = grid.width() / 128.0;
        let begin = self.section * 128;
        painter.rect_filled(rect, 4.0, Color32::from_rgb(16, 19, 25));
        for pitch in base..base + 24 {
            let y =
                grid.bottom() - f32::from(i16::try_from(pitch - base + 1).expect("pitch")) * row;
            let black = [1, 3, 6, 8, 10].contains(&pitch.rem_euclid(12));
            let band = egui::Rect::from_min_size(
                egui::pos2(grid.left(), y),
                egui::vec2(grid.width(), row),
            );
            painter.rect_filled(band, 0.0, Color32::from_gray(if black { 22 } else { 29 }));
            if pitch % 12 == 0 {
                painter.text(
                    egui::pos2(rect.left() + 6.0, y),
                    egui::Align2::LEFT_TOP,
                    format!("C{}", pitch / 12 - 1),
                    egui::FontId::monospace(10.0),
                    Color32::GRAY,
                );
            }
        }
        for tick in 0..=128 {
            let x = grid.left() + display(tick) * step;
            painter.line_segment(
                [egui::pos2(x, grid.top()), egui::pos2(x, grid.bottom())],
                egui::Stroke::new(
                    if tick % 16 == 0 { 1.0 } else { 0.5 },
                    Color32::from_gray(if tick % 16 == 0 { 85 } else { 40 }),
                ),
            );
            if tick < 128 && tick % 16 == 0 {
                painter.text(
                    egui::pos2(x + 4.0, rect.top() + 4.0),
                    egui::Align2::LEFT_TOP,
                    format!("{:02}", (begin + tick) / 16 + 1),
                    egui::FontId::monospace(11.0),
                    Color32::LIGHT_GRAY,
                );
            }
        }
        for (index, note) in self.scores[self.selected][self.edit_track]
            .iter()
            .enumerate()
        {
            if note.start + note.len <= begin
                || note.start >= begin + 128
                || !(base..base + 24).contains(&note.pitch)
            {
                continue;
            }
            let x = grid.left() + display(note.start.saturating_sub(begin)) * step;
            let width =
                display((note.start + note.len).min(begin + 128) - note.start.max(begin)) * step;
            let y = grid.bottom()
                - f32::from(i16::try_from(note.pitch - base + 1).expect("pitch")) * row;
            let color = if self.selected_note == Some(index) {
                Color32::WHITE
            } else {
                TRACK_COLORS[self.edit_track]
            };
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(x + 1.0, y + 1.0),
                    egui::vec2((width - 2.0).max(2.0), row - 2.0),
                ),
                2.0,
                color,
            );
        }
        let playhead = self.playhead_bar() * 16.0 - display(begin);
        if self.transport.is_some() && (0.0..128.0).contains(&playhead) {
            let x = grid.left() + playhead * step;
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(2.0, ACCENT),
            );
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
