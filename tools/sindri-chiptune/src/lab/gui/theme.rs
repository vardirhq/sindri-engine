//! The Sound Lab's look: the editor's forge palette, Inter, and a few shared surfaces.
//!
//! The values follow `editor/src/ui/theme.rs` so the tool reads as part of
//! Sindri; the tool cannot depend on the editor crate, so they are repeated here.
use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontFamily, FontId, Margin, RichText, Stroke,
    StrokeKind, TextStyle, Vec2,
};
use egui_material_icons::MaterialIcon;
use std::sync::Arc;

pub(super) const INK: Color32 = Color32::from_rgb(8, 10, 13);
pub(super) const PANEL: Color32 = Color32::from_rgb(15, 19, 23);
pub(super) const HEADER: Color32 = Color32::from_rgb(18, 22, 27);
pub(super) const RAISED: Color32 = Color32::from_rgb(22, 27, 33);
pub(super) const WELL: Color32 = Color32::from_rgb(11, 14, 18);
pub(super) const LINE: Color32 = Color32::from_rgb(37, 43, 50);
pub(super) const LINE_SOFT: Color32 = Color32::from_rgb(27, 32, 38);
pub(super) const TEXT: Color32 = Color32::from_rgb(232, 228, 220);
pub(super) const MUTED: Color32 = Color32::from_rgb(152, 160, 168);
pub(super) const FAINT: Color32 = Color32::from_rgb(104, 113, 122);
pub(super) const FORGE: Color32 = Color32::from_rgb(233, 163, 58);
pub(super) const FORGE_BRIGHT: Color32 = Color32::from_rgb(255, 190, 85);
pub(super) const FORGE_DIM: Color32 = Color32::from_rgb(138, 98, 32);
pub(super) const EMBER: Color32 = Color32::from_rgb(46, 34, 17);
pub(super) const DANGER: Color32 = Color32::from_rgb(255, 138, 148);
pub(super) const SUCCESS: Color32 = Color32::from_rgb(121, 182, 138);

/// Bass, arpeggio, lead and percussion, wherever a voice is named.
pub(super) const VOICES: [Color32; 4] = [
    Color32::from_rgb(110, 168, 230),
    Color32::from_rgb(170, 140, 230),
    Color32::from_rgb(96, 214, 170),
    Color32::from_rgb(230, 150, 105),
];

// Shared with the editor rather than copied, so the two cannot drift apart.
const INTER: &[u8] = include_bytes!("../../../../../editor/assets/Inter.ttf");

pub(super) fn install(context: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("inter".into(), Arc::new(FontData::from_static(INTER)));
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "inter".into());
    context.set_fonts(fonts);
    egui_material_icons::initialize(context);
    context.set_theme(egui::Theme::Dark);
    context.all_styles_mut(apply);
}

fn apply(style: &mut egui::Style) {
    let radius = CornerRadius::same(4);
    style.spacing.item_spacing = Vec2::new(6.0, 6.0);
    style.spacing.button_padding = Vec2::new(9.0, 4.0);
    style.spacing.interact_size.y = 24.0;
    style.spacing.slider_width = 150.0;
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.floating = true;
    for (text, size) in [
        (TextStyle::Body, 13.0),
        (TextStyle::Button, 13.0),
        (TextStyle::Small, 11.0),
        (TextStyle::Heading, 17.0),
    ] {
        style
            .text_styles
            .insert(text, FontId::new(size, FontFamily::Proportional));
    }
    let v = &mut style.visuals;
    v.panel_fill = PANEL;
    v.window_fill = RAISED;
    v.window_stroke = Stroke::new(1.0, LINE);
    v.extreme_bg_color = WELL;
    v.faint_bg_color = RAISED;
    v.selection.bg_fill = EMBER;
    v.selection.stroke = Stroke::new(1.0, FORGE);
    v.hyperlink_color = FORGE;
    v.weak_text_color = Some(FAINT);
    v.override_text_color = None;
    v.disabled_alpha = 0.42;
    v.slider_trailing_fill = true;
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.bg_stroke = Stroke::new(1.0, LINE_SOFT);
    w.noninteractive.fg_stroke = Stroke::new(1.0, MUTED);
    w.inactive.bg_fill = RAISED;
    w.inactive.weak_bg_fill = RAISED;
    w.inactive.bg_stroke = Stroke::new(1.0, LINE_SOFT);
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.hovered.bg_fill = Color32::from_rgb(31, 37, 45);
    w.hovered.weak_bg_fill = Color32::from_rgb(31, 37, 45);
    w.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(58, 67, 77));
    w.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    w.hovered.expansion = 0.0;
    w.active.bg_fill = EMBER;
    w.active.weak_bg_fill = EMBER;
    w.active.bg_stroke = Stroke::new(1.0, FORGE);
    w.active.fg_stroke = Stroke::new(1.0, FORGE_BRIGHT);
    w.active.expansion = 0.0;
    w.open.bg_fill = EMBER;
    w.open.weak_bg_fill = EMBER;
    w.open.bg_stroke = Stroke::new(1.0, FORGE_DIM);
    for state in [
        &mut w.noninteractive,
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        state.corner_radius = radius;
    }
}

/// An icon followed by a label, in one string the proportional font can draw.
pub(super) fn label(icon: MaterialIcon, text: &str) -> String {
    let glyph: &str = icon.into();
    if text.is_empty() {
        glyph.to_owned()
    } else {
        format!("{glyph}  {text}")
    }
}

/// A small uppercase caption above a group of controls.
pub(super) fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(10.5).strong().color(FAINT));
}

/// A raised surface with a colored title rule.
pub(super) fn card<R>(
    ui: &mut egui::Ui,
    title: &str,
    accent: Color32,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    egui::Frame::new()
        .fill(RAISED)
        .stroke(Stroke::new(1.0, LINE_SOFT))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rule, _) = ui.allocate_exact_size(Vec2::new(3.0, 14.0), egui::Sense::hover());
                ui.painter().rect_filled(rule, 1.5, accent);
                ui.label(RichText::new(title).size(11.5).strong().color(TEXT));
            });
            ui.add_space(4.0);
            add(ui)
        })
        .inner
}

/// The one bright action in a group.
pub(super) fn primary(text: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(text.into().color(INK).strong())
        .fill(FORGE)
        .stroke(Stroke::new(1.0, FORGE_BRIGHT))
        .corner_radius(CornerRadius::same(5))
}

/// A toggle that lights in `accent` when on.
pub(super) fn toggle(ui: &mut egui::Ui, on: &mut bool, text: &str, accent: Color32) -> bool {
    let fill = if *on {
        accent.gamma_multiply(0.25)
    } else {
        RAISED
    };
    let stroke = if *on { accent } else { LINE_SOFT };
    let color = if *on { accent } else { MUTED };
    let clicked = ui
        .add(
            egui::Button::new(RichText::new(text).color(color).strong().size(11.0))
                .fill(fill)
                .stroke(Stroke::new(1.0, stroke))
                .min_size(Vec2::new(26.0, 22.0)),
        )
        .clicked();
    if clicked {
        *on = !*on;
    }
    clicked
}

/// A segmented choice: one lit button among several.
pub(super) fn segment(
    ui: &mut egui::Ui,
    selected: bool,
    text: impl Into<egui::WidgetText>,
) -> bool {
    let button = egui::Button::new(text)
        .selected(selected)
        .fill(if selected { EMBER } else { WELL })
        .stroke(Stroke::new(1.0, if selected { FORGE } else { LINE_SOFT }));
    ui.add(button).clicked()
}

/// Outlines `rect` as a well something is drawn into.
pub(super) fn well(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_filled(rect, 6.0, WELL);
    painter.rect_stroke(rect, 6.0, Stroke::new(1.0, LINE_SOFT), StrokeKind::Inside);
}
