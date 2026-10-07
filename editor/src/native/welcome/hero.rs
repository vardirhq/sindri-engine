//! The banner across the top of the welcome window.
//!
//! Drawn rather than loaded: the editor has no image loading, and a banner
//! made of a gradient and a few polygons costs nothing to ship, scales to any
//! window width, and stays in the theme's palette. The scene is a dusk range —
//! Sindri is the dwarf who forged, and a forge glow behind mountains is the
//! brand without a word of copy.

use eframe::egui::{self, Align2, Color32, Mesh, Pos2, Rect, Sense, Vec2};

use crate::ui::theme::{color, hairline, text};

pub(super) const HEIGHT: f32 = 184.0;

/// Sky colours, top to bottom.
const SKY_TOP: Color32 = Color32::from_rgb(12, 16, 26);
const SKY_HORIZON: Color32 = Color32::from_rgb(58, 40, 34);

/// Ridges from farthest to nearest: base height as a fraction of the banner,
/// how far they rise and fall, a phase so no two line up, and their colour.
const RIDGES: [(f32, f32, f32, Color32); 3] = [
    (0.40, 0.36, 0.0, Color32::from_rgb(66, 56, 72)),
    (0.58, 0.30, 1.7, Color32::from_rgb(36, 34, 48)),
    (0.80, 0.22, 3.1, Color32::from_rgb(16, 19, 26)),
];

pub(super) fn draw(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), HEIGHT), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.add(vertical_gradient(rect, SKY_TOP, SKY_HORIZON));
    painter.circle_filled(
        Pos2::new(rect.right() - rect.width() * 0.18, rect.top() + 52.0),
        26.0,
        Color32::from_rgb(222, 206, 178),
    );
    painter.circle_filled(
        Pos2::new(rect.right() - rect.width() * 0.18, rect.top() + 52.0),
        40.0,
        Color32::from_rgba_unmultiplied(222, 206, 178, 6),
    );
    for &(base, swing, phase, fill) in &RIDGES {
        painter.add(ridge(rect, base, swing, phase, fill));
    }
    // A fade from the window's own ink on the left, so the copy sits on
    // something quiet however the ridges happen to fall at this width.
    painter.add(horizontal_fade(
        Rect::from_min_max(
            rect.min,
            Pos2::new(rect.left() + rect.width() * 0.5, rect.bottom()),
        ),
        color::INK,
    ));
    painter.hline(rect.x_range(), rect.bottom() - 0.5, hairline());

    let left = rect.left() + 32.0;
    logo(
        &painter,
        Rect::from_min_size(Pos2::new(left, rect.top() + 30.0), Vec2::new(64.0, 54.0)),
    );
    painter.text(
        Pos2::new(left + 84.0, rect.top() + 26.0),
        Align2::LEFT_TOP,
        "Sindri",
        egui::FontId::proportional(38.0),
        color::TEXT,
    );
    painter.text(
        Pos2::new(left + 86.0, rect.top() + 74.0),
        Align2::LEFT_TOP,
        "Build worlds. Give them rules. Make them playable.",
        egui::FontId::proportional(17.0),
        color::TEXT,
    );
    painter.text(
        Pos2::new(left, rect.top() + 118.0),
        Align2::LEFT_TOP,
        "A Rust-powered 2D + 3D engine with a native editor,",
        egui::FontId::proportional(text::BODY + 1.0),
        color::TEXT_MUTED,
    );
    painter.text(
        Pos2::new(left, rect.top() + 140.0),
        Align2::LEFT_TOP,
        "the Decay gameplay language, and responsive Weave UI.",
        egui::FontId::proportional(text::BODY + 1.0),
        color::TEXT_MUTED,
    );
}

/// The mark: a peak in forge amber with a paler one behind it, capped in snow.
fn logo(painter: &egui::Painter, area: Rect) {
    let base = area.bottom();
    let back = [
        Pos2::new(area.left() + area.width() * 0.38, base),
        Pos2::new(
            area.left() + area.width() * 0.70,
            area.top() + area.height() * 0.22,
        ),
        Pos2::new(area.right(), base),
    ];
    let front = [
        Pos2::new(area.left(), base),
        Pos2::new(area.left() + area.width() * 0.40, area.top()),
        Pos2::new(area.left() + area.width() * 0.80, base),
    ];
    let cap = [
        Pos2::new(
            area.left() + area.width() * 0.29,
            area.top() + area.height() * 0.27,
        ),
        Pos2::new(area.left() + area.width() * 0.40, area.top()),
        Pos2::new(
            area.left() + area.width() * 0.51,
            area.top() + area.height() * 0.27,
        ),
        Pos2::new(
            area.left() + area.width() * 0.40,
            area.top() + area.height() * 0.36,
        ),
    ];
    for (points, fill) in [
        (back.to_vec(), Color32::from_rgb(138, 104, 52)),
        (front.to_vec(), color::FORGE),
        (cap.to_vec(), Color32::from_rgb(246, 238, 222)),
    ] {
        painter.add(egui::Shape::convex_polygon(
            points,
            fill,
            egui::Stroke::NONE,
        ));
    }
}

fn vertical_gradient(rect: Rect, top: Color32, bottom: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    mesh
}

/// Opaque at the left edge, clear at the right.
fn horizontal_fade(rect: Rect, ink: Color32) -> Mesh {
    let clear = Color32::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), 0);
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), ink);
    mesh.colored_vertex(rect.right_top(), clear);
    mesh.colored_vertex(rect.left_bottom(), ink);
    mesh.colored_vertex(rect.right_bottom(), clear);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    mesh
}

/// A mountain ridge filled down to the bottom of the banner.
///
/// The skyline is a sum of three sines at unrelated frequencies, which reads
/// as peaks rather than waves and comes out the same on every launch.
fn ridge(rect: Rect, base: f32, swing: f32, phase: f32, fill: Color32) -> Mesh {
    const STEPS: u16 = 64;
    let mut mesh = Mesh::default();
    for step in 0..=STEPS {
        let t = f32::from(step) / f32::from(STEPS);
        let x = rect.left() + rect.width() * t;
        let angle = t * 9.0 + phase;
        let lift =
            (angle.sin() * 0.55 + (angle * 2.3).sin() * 0.3 + (angle * 5.1).sin() * 0.15).abs();
        let y = rect.top() + rect.height() * (base + swing * (0.5 - lift));
        mesh.colored_vertex(Pos2::new(x, y), fill);
        mesh.colored_vertex(Pos2::new(x, rect.bottom()), fill);
        if step > 0 {
            let top = u32::from(step) * 2;
            mesh.add_triangle(top - 2, top, top - 1);
            mesh.add_triangle(top, top + 1, top - 1);
        }
    }
    mesh
}
