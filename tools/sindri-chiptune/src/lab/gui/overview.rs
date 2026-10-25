//! The whole song at a glance: four sections, three voices, and the playhead.
use super::{ComposerApp, TRACK_COLORS, theme};
use crate::lab::{BARS, STEPS, live::Command};
use eframe::egui::{self, Pos2, Rect, RichText, Stroke, StrokeKind, Vec2};

const SECTIONS: [&str; 4] = ["INTRO", "THEME", "RISE", "RETURN"];
const BARS_PER_SECTION: usize = BARS / SECTIONS.len();
const TRACKS: [&str; 3] = ["Bass", "Arpeggio", "Lead"];
const TRACK_COLUMN_WIDTH: f32 = 158.0;
const TRACK_COLUMN_COMPACT: f32 = 132.0;
const TRACK_ROW_HEIGHT: f32 = 24.0;
const ARRANGEMENT_HEIGHT: f32 = TRACK_ROW_HEIGHT * 3.0;

fn x_of(step: usize, rect: Rect) -> f32 {
    rect.left()
        + f32::from(u16::try_from(step).expect("score step"))
            / f32::from(u16::try_from(STEPS).expect("steps"))
            * rect.width()
}

impl ComposerApp {
    pub(super) fn overview(&mut self, ui: &mut egui::Ui) {
        self.section_headers(ui);
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            self.track_headers(ui);
            ui.add_space(4.0);
            self.overview_waveform(ui);
        });
        ui.add_space(2.0);
        ui.label(
            RichText::new(
                "Drums are generated from mood and density and play under every section.",
            )
            .size(10.5)
            .color(theme::FAINT),
        );
    }

    pub(super) fn track_headers(&mut self, ui: &mut egui::Ui) {
        let selected = self.selected;
        let index = self.edit_track;
        let column_width = if self.compact {
            TRACK_COLUMN_COMPACT
        } else {
            TRACK_COLUMN_WIDTH
        };
        ui.allocate_ui_with_layout(
            Vec2::new(column_width, ARRANGEMENT_HEIGHT),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(1.0, 2.0);
                if self.compact {
                    ui.spacing_mut().button_padding.x = 4.0;
                }
                for (track, name) in TRACKS.iter().enumerate() {
                    let color = TRACK_COLORS[track];
                    let name = if self.compact && track == 1 {
                        "Arp"
                    } else {
                        *name
                    };
                    let focused = index == track;
                    let (fill, stroke) = if focused {
                        (color.gamma_multiply(0.16), color)
                    } else {
                        (theme::WELL, theme::LINE_SOFT)
                    };
                    let sound = &mut self.settings.rack.tracks[track];
                    let locked = &mut self.track_locks[selected][track];
                    let select = ui
                        .horizontal(|ui| {
                            let label = egui::Button::new(
                                RichText::new(name)
                                    .color(color)
                                    .strong()
                                    .size(if self.compact { 9.5 } else { 10.5 }),
                            )
                            .fill(fill)
                            .stroke(Stroke::new(1.0, stroke))
                            .min_size(Vec2::new(if self.compact { 34.0 } else { 54.0 }, 20.0));
                            let select = ui.add(label).clicked();
                            theme::toggle(ui, locked, "Lock", theme::FORGE);
                            theme::toggle(ui, &mut sound.muted, "M", theme::DANGER);
                            theme::toggle(ui, &mut sound.solo, "S", theme::FORGE_BRIGHT);
                            select
                        })
                        .inner;
                    if select {
                        self.edit_track = track;
                        self.fit_pitch_range();
                        self.selected_note = None;
                    }
                }
            },
        );
    }

    fn overview_waveform(&mut self, ui: &mut egui::Ui) {
        let size = Vec2::new(ui.available_width(), ARRANGEMENT_HEIGHT);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        let painter = ui.painter_at(rect);
        theme::well(&painter, rect);
        let body = Rect::from_min_max(
            rect.min + Vec2::new(0.0, 4.0),
            rect.max - Vec2::new(0.0, 4.0),
        );
        for section in 0..SECTIONS.len() {
            let band = Rect::from_min_max(
                Pos2::new(x_of(section * BARS_PER_SECTION * STEPS, rect), rect.top()),
                Pos2::new(
                    x_of((section + 1) * BARS_PER_SECTION * STEPS, rect),
                    rect.bottom(),
                ),
            );
            let current = self.section == section;
            if current {
                painter.rect_filled(band.shrink(1.0), 5.0, theme::EMBER.gamma_multiply(0.7));
                painter.rect_stroke(
                    band.shrink(1.0),
                    5.0,
                    Stroke::new(1.0, theme::FORGE_DIM),
                    StrokeKind::Inside,
                );
            }
            if section > 0 {
                painter.line_segment(
                    [
                        Pos2::new(band.left(), rect.top() + 4.0),
                        Pos2::new(band.left(), rect.bottom() - 4.0),
                    ],
                    Stroke::new(1.0, theme::LINE),
                );
            }
        }
        for bar in 1..BARS {
            if bar % 8 != 0 {
                let x = x_of(bar * 16, rect);
                painter.line_segment(
                    [Pos2::new(x, body.top() + 2.0), Pos2::new(x, body.bottom())],
                    Stroke::new(1.0, theme::LINE_SOFT.gamma_multiply(0.5)),
                );
            }
        }
        self.voices(&painter, rect, body);
        if self.transport.is_some() {
            // The playhead is drawn in bars; this converts it to the overview's width.
            let at = self.playhead_bar() / 32.0;
            let x = rect.left() + at * rect.width();
            painter.line_segment(
                [
                    Pos2::new(x, rect.top() + 2.0),
                    Pos2::new(x, rect.bottom() - 2.0),
                ],
                Stroke::new(2.0, theme::FORGE_BRIGHT),
            );
        }
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let at = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 0.999);
            // Display coordinates: a fraction of the 32-bar song.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let bar = (at * 32.0) as usize;
            self.section = bar / BARS_PER_SECTION;
            if let Some(transport) = &self.transport {
                let _ = transport.tx.send(Command::Seek(bar));
            }
        }
        response.on_hover_text("Click a section to edit it; while playing, click to jump there");
    }

    pub(super) fn section_headers(&mut self, ui: &mut egui::Ui) {
        let column_width = if self.compact {
            TRACK_COLUMN_COMPACT
        } else {
            TRACK_COLUMN_WIDTH
        };
        ui.horizontal(|ui| {
            if self.compact {
                ui.spacing_mut().item_spacing.x = 3.0;
                ui.spacing_mut().button_padding.x = 4.0;
            }
            let gap = ui.spacing().item_spacing.x;
            let width = (ui.available_width() - column_width - gap * 4.0) / 4.0;
            ui.add_space(column_width);
            for (section, name) in SECTIONS.iter().enumerate() {
                let first_bar = section * BARS_PER_SECTION + 1;
                let last_bar = first_bar + BARS_PER_SECTION - 1;
                let label = RichText::new(format!("{name}\n{first_bar}–{last_bar}"))
                    .size(if self.compact { 9.0 } else { 10.5 })
                    .strong();
                let (fill, stroke) = if self.section == section {
                    (theme::EMBER, theme::FORGE)
                } else {
                    (theme::WELL, theme::LINE_SOFT)
                };
                let button = egui::Button::new(label)
                    .fill(fill)
                    .stroke(Stroke::new(1.0, stroke));
                if ui.add_sized(Vec2::new(width, 34.0), button).clicked() {
                    self.section = section;
                    if let Some(transport) = &self.transport {
                        let _ = transport.tx.send(Command::Seek(section * BARS_PER_SECTION));
                    }
                }
            }
        });
    }

    /// Each voice's notes as marks in its own lane, scaled to that voice's range.
    fn voices(&self, painter: &egui::Painter, rect: Rect, body: Rect) {
        let lane = body.height() / 3.0;
        for (voice, notes) in self.scores[self.selected].iter().enumerate() {
            // Lead on top, bass at the bottom, as a score reads.
            let top = body.top() + lane * f32::from(u8::try_from(voice).expect("voice"));
            let (low, high) = notes
                .iter()
                .fold((127, 0), |(lo, hi), n| (n.pitch.min(lo), n.pitch.max(hi)));
            let span = f32::from(i16::try_from((high - low).max(1)).expect("pitch span"));
            let color = TRACK_COLORS[voice];
            let faded = if voice == self.edit_track { 1.0 } else { 0.55 };
            for note in notes {
                let height = f32::from(i16::try_from(note.pitch - low).expect("pitch")) / span;
                let y = top + lane - 4.0 - height * (lane - 8.0);
                let mark = Rect::from_min_max(
                    Pos2::new(x_of(note.start, rect), y - 1.5),
                    Pos2::new(
                        x_of(note.start + note.len, rect).max(x_of(note.start, rect) + 2.0),
                        y + 1.5,
                    ),
                );
                painter.rect_filled(mark, 1.0, color.gamma_multiply(faded));
            }
        }
    }
}
