//! The Profiler panel: each frame of Play as a bar of its phases, and what
//! the frames — or the one pointed at — spent their time on.

use std::time::Duration;

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use crate::profiler::{KEPT, Phase, Profiler, Summary};
use crate::ui::icons;
use crate::ui::theme::{color, metric, text};
use crate::ui::widgets::{button, panel};

use super::EditorApp;

/// A frame at 60 frames a second, which the chart marks.
const BUDGET: Duration = Duration::from_micros(16_667);
const CHART_HEIGHT: f32 = 112.0;

/// The color a phase is drawn in, in the chart and beside its row.
const fn phase_color(phase: Phase) -> Color32 {
    match phase {
        Phase::Upkeep => color::FORGE_DIM,
        Phase::Effects => color::AXIS_Z_DIM,
        Phase::Physics => color::AXIS_Z,
        Phase::ScreenUi => color::AXIS_Y_DIM,
        Phase::Scripts => color::FORGE,
        Phase::Animation => color::AXIS_Y,
        Phase::Cameras => color::AXIS_X_DIM,
        Phase::Placement => color::EMBER_FAINT,
        Phase::Presentation => color::AXIS_X,
        Phase::Extraction => color::TEXT_MUTED,
        Phase::Encoding => color::TEXT_FAINT,
        Phase::Gpu => color::LINE_SOFT,
        Phase::SceneView => color::FORGE_BRIGHT,
        Phase::Panels => color::EMBER,
        Phase::Paint => color::LINE,
        Phase::Waiting => color::WELL,
    }
}

fn millis(time: Duration) -> String {
    format!("{:.2} ms", time.as_secs_f64() * 1000.0)
}

impl EditorApp {
    /// The Profiler panel.
    pub(super) fn profiler_body(&mut self, ui: &mut egui::Ui) {
        let asked = profiler_panel(ui, &self.profiler);
        if asked.clear {
            self.profiler.clear();
        }
        if asked.toggle_editing {
            let on = !self.profiler.while_editing();
            self.profiler.set_while_editing(on);
        }
    }
}

/// What the panel's toolbar was asked to do.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Asked {
    pub(super) clear: bool,
    pub(super) toggle_editing: bool,
}

/// Draws the panel; answers what its toolbar was asked to do.
pub(super) fn profiler_panel(ui: &mut egui::Ui, profiler: &Profiler) -> Asked {
    let frames = profiler.frames();
    let summary = profiler.summary();
    let mut asked = Asked::default();
    ui.horizontal(|ui| {
        ui.set_height(metric::TOOLBAR_HEIGHT);
        ui.add_space(metric::GUTTER);
        let said = if frames.is_empty() {
            "No frames yet".to_owned()
        } else {
            let rate = if summary.interval.is_zero() {
                0.0
            } else {
                1.0 / summary.interval.as_secs_f64()
            };
            format!(
                "{} frames · {:.0} fps · working {} on average, {} at worst",
                summary.frames,
                rate,
                millis(summary.average),
                millis(summary.worst)
            )
        };
        ui.label(
            RichText::new(said)
                .size(text::LABEL)
                .color(color::TEXT_MUTED),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(metric::GUTTER);
            if button::icon(
                ui,
                icons::REMOVE,
                false,
                "Forget the frames recorded so far",
            )
            .clicked()
            {
                asked.clear = true;
            }
            if button::icon(
                ui,
                icons::INSPECTOR,
                profiler.while_editing(),
                "Record frames while editing too, not only while playing",
            )
            .clicked()
            {
                asked.toggle_editing = true;
            }
        });
    });
    panel::rule_tight(ui);
    if frames.is_empty() {
        panel::empty_state(
            ui,
            icons::PROFILER,
            "Nothing measured yet",
            "Press Play: each frame's steps, views, panels and painting are timed here, and each script's share of them.",
        );
        return asked;
    }
    let pinned_id = ui.id().with("profiler pinned frame");
    let mut pinned: Option<usize> = ui.data(|data| data.get_temp(pinned_id));
    if pinned.is_some_and(|at| at >= frames.len()) {
        pinned = None;
    }
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            panel::body(ui, |ui| {
                let pointed = chart(ui, profiler, &summary, &mut pinned);
                ui.data_mut(|data| data.insert_temp(pinned_id, pinned));
                ui.add_space(metric::GROUP_GAP);
                let shown = pointed.or(pinned).and_then(|at| frames.get(at));
                if let Some(frame) = shown {
                    heading(
                        ui,
                        &format!(
                            "One frame: {} working in {} step{}",
                            millis(frame.work()),
                            frame.steps,
                            if frame.steps == 1 { "" } else { "s" }
                        ),
                    );
                    phases(ui, |phase| frame.phase(phase), frame.work());
                    scripts(ui, &Summary::of(std::iter::once(frame)));
                } else {
                    heading(ui, "Average frame");
                    phases(ui, |phase| summary.phase(phase), summary.average);
                    scripts(ui, &summary);
                }
            });
        });
    asked
}

fn heading(ui: &mut egui::Ui, words: &str) {
    ui.label(RichText::new(words).size(text::HEADING).color(color::TEXT));
    ui.add_space(4.0);
}

/// The frames' work as stacked bars, newest on the right, with the 60 Hz
/// budget marked. Waiting for the GPU or the display is left out: at a
/// display's rate it fills every bar up to the refresh interval, which says
/// nothing about the editor. Answers the frame under the pointer; a click
/// pins one, and a click on the same one again lets it go.
fn chart(
    ui: &mut egui::Ui,
    profiler: &Profiler,
    summary: &Summary,
    pinned: &mut Option<usize>,
) -> Option<usize> {
    let frames = profiler.frames();
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), CHART_HEIGHT),
        Sense::click(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, metric::RADIUS, color::WELL);
    let ceiling = summary.worst.max(BUDGET).as_secs_f32() * 1.15;
    #[allow(clippy::cast_precision_loss)]
    let slot = rect.width() / KEPT as f32;
    let height_of = |time: Duration| time.as_secs_f32() / ceiling * rect.height();
    let first = KEPT - frames.len();
    #[allow(clippy::cast_precision_loss)]
    let column = |at: usize| rect.left() + (first + at) as f32 * slot;
    let pointed = response.hover_pos().and_then(|pointer| {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let index = ((pointer.x - rect.left()) / slot).floor().max(0.0) as usize;
        index.checked_sub(first).filter(|at| *at < frames.len())
    });
    for (at, frame) in frames.iter().enumerate() {
        let left = column(at);
        let mut bottom = rect.bottom();
        let lit = Some(at) == pointed || Some(at) == *pinned;
        for phase in Phase::ALL.into_iter().filter(|phase| phase.is_work()) {
            let tall = height_of(frame.phase(phase));
            if tall <= 0.0 {
                continue;
            }
            let fill = phase_color(phase);
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(left, bottom - tall),
                    Pos2::new(left + (slot - 0.5).max(1.0), bottom),
                ),
                0.0,
                if lit { fill } else { fill.gamma_multiply(0.75) },
            );
            bottom -= tall;
        }
        if lit {
            painter.rect_stroke(
                Rect::from_min_max(
                    Pos2::new(left - 1.0, rect.top()),
                    Pos2::new(left + slot + 1.0, rect.bottom()),
                ),
                0.0,
                Stroke::new(1.0, color::FORGE_DIM),
                egui::StrokeKind::Inside,
            );
        }
    }
    let budget_y = rect.bottom() - height_of(BUDGET);
    painter.hline(rect.x_range(), budget_y, Stroke::new(1.0, color::WARNING));
    painter.text(
        Pos2::new(rect.left() + 4.0, budget_y - 2.0),
        egui::Align2::LEFT_BOTTOM,
        "16.7 ms · 60 fps",
        egui::FontId::proportional(text::NOTE),
        color::WARNING,
    );
    if response.clicked() {
        *pinned = if pointed == *pinned { None } else { pointed };
    }
    pointed
}

/// Each phase's time, with its color and its share of the frame's work.
/// Waiting for the GPU or the display is no share of that, so those rows
/// have none.
fn phases(ui: &mut egui::Ui, time_of: impl Fn(Phase) -> Duration, total: Duration) {
    egui::Grid::new("profiler phases")
        .num_columns(3)
        .spacing([metric::GROUP_GAP, 3.0])
        .show(ui, |ui| {
            for phase in Phase::ALL {
                let time = time_of(phase);
                ui.horizontal(|ui| {
                    let (swatch, _) = ui.allocate_exact_size(Vec2::splat(9.0), Sense::hover());
                    ui.painter().rect_filled(swatch, 2.0, phase_color(phase));
                    ui.label(
                        RichText::new(phase.label())
                            .size(text::LABEL)
                            .color(color::TEXT),
                    );
                });
                ui.label(
                    RichText::new(millis(time))
                        .size(text::LABEL)
                        .monospace()
                        .color(color::TEXT_MUTED),
                );
                let share = if total.is_zero() || !phase.is_work() {
                    String::new()
                } else {
                    format!("{:.0}%", time.as_secs_f64() / total.as_secs_f64() * 100.0)
                };
                ui.label(
                    RichText::new(share)
                        .size(text::LABEL)
                        .color(color::TEXT_FAINT),
                );
                ui.end_row();
            }
        });
}

/// Each script's time, slowest first.
fn scripts(ui: &mut egui::Ui, summary: &Summary) {
    ui.add_space(metric::GROUP_GAP);
    heading(ui, "Scripts, slowest first");
    if summary.scripts.is_empty() {
        panel::note(ui, "No script ran.");
        return;
    }
    egui::Grid::new("profiler scripts")
        .num_columns(3)
        .striped(true)
        .spacing([metric::GROUP_GAP, 3.0])
        .show(ui, |ui| {
            for timing in &summary.scripts {
                ui.label(
                    RichText::new(&timing.script)
                        .size(text::LABEL)
                        .color(color::TEXT),
                )
                .on_hover_text(&timing.source);
                ui.label(
                    RichText::new(millis(timing.time))
                        .size(text::LABEL)
                        .monospace()
                        .color(color::TEXT_MUTED),
                );
                ui.label(
                    RichText::new(format!(
                        "{} run{}",
                        timing.runs,
                        if timing.runs == 1 { "" } else { "s" }
                    ))
                    .size(text::LABEL)
                    .color(color::TEXT_FAINT),
                );
                ui.end_row();
            }
        });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use eframe::egui;

    use super::profiler_panel;
    use crate::profiler::{Phase, Profiler};

    /// The panel draws with no frames and with some, and Clear is heard.
    #[test]
    fn the_panel_draws_frames_and_clears_them() {
        let context = egui::Context::default();
        egui_material_icons::initialize(&context);
        let mut profiler = Profiler::default();
        let mut cleared = false;
        let draw = |profiler: &Profiler, events: Vec<egui::Event>| {
            let mut asked = false;
            context
                .run_ui(
                    egui::RawInput {
                        events,
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(600.0, 500.0),
                        )),
                        ..Default::default()
                    },
                    |ui| asked = profiler_panel(ui, profiler).clear,
                )
                .drop_without_applying_deltas();
            asked
        };
        draw(&profiler, Vec::new());
        let mut at = std::time::Instant::now();
        for _ in 0..11 {
            profiler.begin(at, Some(Duration::from_millis(4)), true);
            profiler.add(Phase::Scripts, Duration::from_millis(2));
            profiler.step(Vec::new());
            profiler.end(Duration::from_millis(3));
            at += Duration::from_millis(16);
        }
        draw(&profiler, Vec::new());
        // Clear sits at the toolbar's right edge.
        let at = egui::pos2(600.0 - 20.0, 15.0);
        draw(&profiler, vec![egui::Event::PointerMoved(at)]);
        for pressed in [true, false] {
            cleared |= draw(
                &profiler,
                vec![egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::default(),
                }],
            );
        }
        assert!(cleared);
    }
}
