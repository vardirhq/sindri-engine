//! Five aligned layers, generated from the same score and patterns as playback.
use super::{ComposerApp, InspectorTab, TRACK_COLORS, theme};
use crate::lab::{BARS, Note, STEPS, Settings, live::Command, synth};
use eframe::egui::{self, Pos2, Rect, RichText, Stroke, StrokeKind, Vec2};
use egui_material_icons::icons;

const SECTIONS: [&str; 4] = ["INTRO", "THEME", "RISE", "RETURN"];
pub(super) const LAYERS: [&str; 5] = ["Bass", "Arpeggio", "Lead", "Drums", "Chords"];
const ROW: f32 = 30.0;

pub(super) struct ArrangementPreview {
    settings: Settings,
    bass: Vec<Note>,
    generated: [Vec<Note>; 2],
}

fn coordinate(value: usize) -> f32 {
    f32::from(u16::try_from(value).expect("score coordinate"))
}

impl ComposerApp {
    pub(super) fn refresh_arrangement(&mut self) {
        let settings = self.selected_settings();
        let bass = &self.scores[self.selected][0];
        if self
            .arrangement
            .as_ref()
            .is_none_or(|old| old.settings != settings || old.bass != *bass)
        {
            self.arrangement = Some(ArrangementPreview {
                generated: synth::generated_layers(&settings, bass),
                settings,
                bass: bass.clone(),
            });
        }
    }

    pub(super) fn layer_notes(&self, layer: usize) -> &[Note] {
        if layer < 3 {
            &self.scores[self.selected][layer]
        } else {
            self.arrangement
                .as_ref()
                .map_or(&[], |preview| &preview.generated[layer - 3])
        }
    }

    pub(super) fn select_layer(&mut self, layer: usize) {
        self.selected_layer = layer;
        if layer < 3 {
            self.edit_track = layer;
        }
        self.selected_note = None;
        self.drag_origin = None;
        self.inspector_tab = InspectorTab::Track;
        self.focus_score_window();
    }

    pub(super) fn overview(&mut self, ui: &mut egui::Ui) {
        self.refresh_arrangement();
        self.section_headers(ui);
        self.track_headers(ui);
    }

    pub(super) fn track_headers(&mut self, ui: &mut egui::Ui) {
        let column = if self.compact { 148.0 } else { 200.0 };
        let width = ui.available_width();
        let (ruler, _) = ui.allocate_exact_size(Vec2::new(width, 19.0), egui::Sense::hover());
        let timeline = Rect::from_min_max(ruler.min + Vec2::new(column, 0.0), ruler.max);
        for bar in (0..BARS).step_by(4) {
            let x = timeline.left() + coordinate(bar) / 32.0 * timeline.width();
            ui.painter().text(
                Pos2::new(x + 3.0, ruler.center().y),
                egui::Align2::LEFT_CENTER,
                (bar + 1).to_string(),
                egui::FontId::monospace(10.0),
                theme::MUTED,
            );
        }
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            for (layer, name) in LAYERS.iter().enumerate() {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(width, ROW), egui::Sense::hover());
                self.arrangement_row(ui, rect, column, layer, name);
            }
        });
    }

    fn arrangement_row(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        column: f32,
        layer: usize,
        name: &str,
    ) {
        let color = TRACK_COLORS[layer];
        let focused = self.selected_layer == layer;
        let header = Rect::from_min_max(
            rect.min,
            Pos2::new(rect.left() + column - 3.0, rect.bottom()),
        );
        let timeline = Rect::from_min_max(Pos2::new(rect.left() + column, rect.top()), rect.max);
        let painter = ui.painter_at(rect);
        painter.rect_filled(
            rect,
            4.0,
            if focused {
                color.gamma_multiply(0.12)
            } else {
                theme::WELL
            },
        );
        painter.rect_filled(
            Rect::from_min_size(rect.min + Vec2::new(4.0, 5.0), Vec2::new(4.0, ROW - 10.0)),
            2.0,
            color,
        );
        let name_rect = Rect::from_min_max(
            header.min + Vec2::new(12.0, 0.0),
            header.max - Vec2::new(71.0, 0.0),
        );
        painter.text(
            name_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            name,
            egui::FontId::proportional(if self.compact { 11.5 } else { 14.0 }),
            theme::TEXT,
        );
        if ui
            .interact(
                name_rect,
                ui.id().with(("layer", layer)),
                egui::Sense::click(),
            )
            .clicked()
        {
            self.select_layer(layer);
        }
        self.layer_keep_button(ui, header, layer);
        self.layer_mix_buttons(ui, header, layer);
        self.paint_layer(&painter, timeline, layer);
        let response = ui.interact(
            timeline,
            ui.id().with(("timeline", layer)),
            egui::Sense::click(),
        );
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            self.select_layer(layer);
            // A pointer fraction is bounded to one of the four song sections.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let section =
                ((pos.x - timeline.left()) / timeline.width() * 4.0).clamp(0.0, 3.0) as usize;
            self.section = section;
            self.roll_page = 0;
            self.focus_score_window();
            if let Some(transport) = &self.transport {
                let _ = transport.tx.send(Command::Seek(section * 8));
            }
        }
        if focused {
            painter.rect_stroke(
                rect.shrink(0.5),
                4.0,
                Stroke::new(1.0, color),
                StrokeKind::Inside,
            );
        }
    }

    fn layer_keep_button(&mut self, ui: &mut egui::Ui, header: Rect, layer: usize) {
        let painter = ui.painter_at(header);
        if layer < 3 {
            let locked = &mut self.track_locks[self.selected][layer];
            let icon = if *locked {
                icons::ICON_LOCK
            } else {
                icons::ICON_LOCK_OPEN
            };
            let rect = Rect::from_min_size(
                Pos2::new(header.right() - 72.0, header.top()),
                Vec2::new(24.0, ROW),
            );
            let glyph: &str = icon.into();
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                glyph,
                egui::FontId::proportional(15.0),
                if *locked { theme::FORGE } else { theme::FAINT },
            );
            if ui
                .interact(rect, ui.id().with(("keep", layer)), egui::Sense::click())
                .on_hover_text("Keep this track when generating")
                .clicked()
            {
                *locked = !*locked;
            }
        } else {
            let glyph: &str = icons::ICON_LINK.into();
            painter.text(
                Pos2::new(header.right() - 60.0, header.center().y),
                egui::Align2::CENTER_CENTER,
                glyph,
                egui::FontId::proportional(14.0),
                theme::FAINT,
            );
        }
    }

    fn layer_mix_buttons(&mut self, ui: &mut egui::Ui, header: Rect, layer: usize) {
        let sound = if layer == 4 {
            &mut self.settings.rack.pad
        } else {
            &mut self.settings.rack.tracks[layer]
        };
        for (index, (label, state)) in [("M", &mut sound.muted), ("S", &mut sound.solo)]
            .into_iter()
            .enumerate()
        {
            let x = header.right() - 48.0 + coordinate(index) * 24.0;
            let rect =
                Rect::from_min_size(Pos2::new(x, header.top() + 3.0), Vec2::new(22.0, ROW - 6.0));
            ui.painter()
                .rect_stroke(rect, 3.0, Stroke::new(1.0, theme::LINE), StrokeKind::Inside);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(12.0),
                if *state { theme::FORGE } else { theme::MUTED },
            );
            if ui
                .interact(
                    rect,
                    ui.id().with(("mix", layer, index)),
                    egui::Sense::click(),
                )
                .clicked()
            {
                *state = !*state;
            }
        }
    }

    fn paint_layer(&self, painter: &egui::Painter, rect: Rect, layer: usize) {
        for bar in 0..BARS {
            let x = rect.left() + coordinate(bar) / 32.0 * rect.width();
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(0.5, theme::LINE),
            );
        }
        let notes = self.layer_notes(layer);
        let low = notes.iter().map(|n| n.pitch).min().unwrap_or(48);
        let high = notes.iter().map(|n| n.pitch).max().unwrap_or(60);
        let span = f32::from(i16::try_from((high - low).max(1)).expect("pitch range"));
        for note in notes {
            let x = rect.left() + coordinate(note.start) / coordinate(STEPS) * rect.width();
            let w = (coordinate(note.len) / coordinate(STEPS) * rect.width()).max(2.0);
            let y = rect.bottom()
                - 5.0
                - f32::from(i16::try_from(note.pitch - low).expect("pitch")) / span * (ROW - 12.0);
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(x, y - 1.5), Vec2::new(w, 3.0)),
                0.5,
                TRACK_COLORS[layer],
            );
        }
        if self.transport.is_some() {
            let x = rect.left() + self.playhead_bar() / 32.0 * rect.width();
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(1.5, theme::FORGE),
            );
        }
    }

    pub(super) fn section_headers(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.spacing_mut().button_padding.x = 2.0;
            let width = (ui.available_width() - 12.0) / 4.0;
            for (section, name) in SECTIONS.iter().enumerate() {
                let color = [
                    TRACK_COLORS[0],
                    theme::FORGE,
                    TRACK_COLORS[1],
                    TRACK_COLORS[4],
                ][section];
                let text =
                    RichText::new(format!("{name}\n{}–{}", section * 8 + 1, section * 8 + 8))
                        .size(11.0);
                let button = egui::Button::new(text)
                    .fill(color.gamma_multiply(if self.section == section { 0.25 } else { 0.10 }))
                    .stroke(Stroke::new(
                        1.0,
                        if self.section == section {
                            color
                        } else {
                            theme::LINE
                        },
                    ));
                if ui.add_sized(Vec2::new(width, 37.0), button).clicked() {
                    self.section = section;
                    self.roll_page = 0;
                    self.focus_score_window();
                    if let Some(transport) = &self.transport {
                        let _ = transport.tx.send(Command::Seek(section * 8));
                    }
                }
            }
        });
    }
}
