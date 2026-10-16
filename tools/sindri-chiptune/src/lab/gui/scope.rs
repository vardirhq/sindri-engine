//! Drawn audio: a live oscilloscope, a static waveform and an envelope sketch.
use super::theme;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

// Drawing coordinates: sample counts and indices are small and display-only.
#[allow(clippy::cast_precision_loss)]
fn fraction(index: usize, count: usize) -> f32 {
    index as f32 / count.max(1) as f32
}

/// A trace of the latest output, glowing in `color`.
pub(super) fn oscilloscope(ui: &mut egui::Ui, samples: &[f32], size: Vec2, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    theme::well(&painter, rect);
    let mid = rect.center().y;
    painter.line_segment(
        [
            Pos2::new(rect.left() + 4.0, mid),
            Pos2::new(rect.right() - 4.0, mid),
        ],
        Stroke::new(1.0, theme::LINE_SOFT),
    );
    if samples.is_empty() {
        return;
    }
    // Start on a rising zero crossing so a steady tone stands still.
    let half = samples.len() / 2;
    let start = (1..half)
        .find(|&i| samples[i - 1] <= 0.0 && samples[i] > 0.0)
        .unwrap_or(0);
    let window = &samples[start..start + half];
    let inner = rect.shrink2(Vec2::new(4.0, 3.0));
    let points: Vec<Pos2> = window
        .iter()
        .enumerate()
        .map(|(i, s)| {
            Pos2::new(
                inner.left() + fraction(i, window.len()) * inner.width(),
                mid - s.clamp(-1.0, 1.0) * inner.height() * 0.5,
            )
        })
        .collect();
    painter.line(points.clone(), Stroke::new(4.0, color.gamma_multiply(0.18)));
    painter.line(points, Stroke::new(1.4, color));
}

/// A peak level meter, filled from the left.
pub(super) fn meter(ui: &mut egui::Ui, samples: &[f32], size: Vec2) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, theme::WELL);
    let peak = samples
        .iter()
        .fold(0.0_f32, |high, s| high.max(s.abs()))
        .min(1.0);
    let lit = Rect::from_min_size(rect.min, Vec2::new(rect.width() * peak, rect.height()));
    let color = if peak > 0.95 {
        theme::DANGER
    } else if peak > 0.7 {
        theme::FORGE_BRIGHT
    } else {
        theme::SUCCESS
    };
    painter.rect_filled(lit, 3.0, color.gamma_multiply(0.85));
}

/// The whole of a short sound, with an optional pitch path and playhead.
pub(super) fn waveform(
    ui: &mut egui::Ui,
    samples: &[f32],
    pitch: &[f32],
    height: f32,
    color: Color32,
    playhead: Option<f32>,
) -> egui::Response {
    let size = Vec2::new(ui.available_width(), height);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let painter = ui.painter_at(rect);
    theme::well(&painter, rect);
    let inner = rect.shrink2(Vec2::new(10.0, 12.0));
    let mid = inner.center().y;
    for i in 1..8 {
        let x = inner.left() + inner.width() * fraction(i, 8);
        painter.line_segment(
            [Pos2::new(x, inner.top()), Pos2::new(x, inner.bottom())],
            Stroke::new(1.0, theme::LINE_SOFT.gamma_multiply(0.6)),
        );
    }
    painter.line_segment(
        [Pos2::new(inner.left(), mid), Pos2::new(inner.right(), mid)],
        Stroke::new(1.0, theme::LINE_SOFT),
    );
    // Display columns are whole points of a finite window.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let columns = inner.width().max(1.0) as usize;
    if !samples.is_empty() {
        for column in 0..columns {
            let from = column * samples.len() / columns;
            let to = ((column + 1) * samples.len() / columns)
                .max(from + 1)
                .min(samples.len());
            let (low, high) = samples[from..to]
                .iter()
                .fold((0.0_f32, 0.0_f32), |(lo, hi), s| (lo.min(*s), hi.max(*s)));
            let x = inner.left() + fraction(column, columns) * inner.width();
            let lit = playhead.is_some_and(|at| fraction(column, columns) <= at);
            let shade = if lit { theme::TEXT } else { color };
            painter.line_segment(
                [
                    Pos2::new(x, mid - high * inner.height() * 0.5),
                    Pos2::new(x, mid - low.min(-0.004) * inner.height() * 0.5),
                ],
                Stroke::new(1.0, shade.gamma_multiply(0.9)),
            );
        }
    }
    if pitch.len() > 1 {
        let points: Vec<Pos2> = pitch
            .iter()
            .enumerate()
            .map(|(i, p)| {
                Pos2::new(
                    inner.left() + fraction(i, pitch.len() - 1) * inner.width(),
                    inner.bottom() - p.clamp(0.0, 1.0) * inner.height(),
                )
            })
            .collect();
        painter.line(
            points,
            Stroke::new(1.5, theme::FORGE_BRIGHT.gamma_multiply(0.85)),
        );
    }
    if let Some(at) = playhead {
        let x = inner.left() + at.clamp(0.0, 1.0) * inner.width();
        painter.line_segment(
            [
                Pos2::new(x, rect.top() + 4.0),
                Pos2::new(x, rect.bottom() - 4.0),
            ],
            Stroke::new(2.0, theme::FORGE_BRIGHT),
        );
    }
    response
}

/// The level shape of an envelope, as a filled outline.
pub(super) fn envelope(ui: &mut egui::Ui, levels: &[f32], color: Color32) {
    let size = Vec2::new(ui.available_width(), 46.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    theme::well(&painter, rect);
    let inner = rect.shrink2(Vec2::new(6.0, 5.0));
    let top = levels.iter().fold(1.0_f32, |high, l| high.max(*l));
    let points: Vec<Pos2> = levels
        .iter()
        .enumerate()
        .map(|(i, level)| {
            Pos2::new(
                inner.left() + fraction(i, levels.len().saturating_sub(1)) * inner.width(),
                inner.bottom() - level / top * inner.height(),
            )
        })
        .collect();
    for pair in points.windows(2) {
        let quad = vec![
            pair[0],
            pair[1],
            Pos2::new(pair[1].x, inner.bottom()),
            Pos2::new(pair[0].x, inner.bottom()),
        ];
        painter.add(egui::Shape::convex_polygon(
            quad,
            color.gamma_multiply(0.18),
            Stroke::NONE,
        ));
    }
    painter.line(points, Stroke::new(1.5, color));
}
