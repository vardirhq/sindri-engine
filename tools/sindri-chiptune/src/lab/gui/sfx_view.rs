//! The Sound FX designer: pick a kind, audition, shape, and deliver to a project.
use super::{ComposerApp, scope, theme};
use crate::lab::sfx::{FORMAT, Kind, Sfx, SfxFile};
use eframe::egui::{self, Color32, RichText, Vec2};
use egui_material_icons::{MaterialIcon, icons};
use web_time::Instant;

const HISTORY: usize = 14;

pub(super) fn icon(kind: Kind) -> MaterialIcon {
    match kind {
        Kind::Coin => icons::ICON_MONETIZATION_ON,
        Kind::Jump => icons::ICON_NORTH_EAST,
        Kind::Laser => icons::ICON_ELECTRIC_BOLT,
        Kind::Explosion => icons::ICON_EXPLOSION,
        Kind::PowerUp => icons::ICON_STAR,
        Kind::Hurt => icons::ICON_HEART_BROKEN,
        Kind::Blip => icons::ICON_RADIO_BUTTON_CHECKED,
        Kind::Thud => icons::ICON_DIRECTIONS_WALK,
    }
}

pub(super) fn tint(kind: Kind) -> Color32 {
    match kind {
        Kind::Coin => Color32::from_rgb(244, 196, 72),
        Kind::Jump => Color32::from_rgb(110, 196, 240),
        Kind::Laser => Color32::from_rgb(240, 98, 146),
        Kind::Explosion => Color32::from_rgb(240, 128, 72),
        Kind::PowerUp => Color32::from_rgb(150, 220, 110),
        Kind::Hurt => Color32::from_rgb(232, 86, 86),
        Kind::Blip => Color32::from_rgb(170, 150, 240),
        Kind::Thud => Color32::from_rgb(190, 160, 128),
    }
}

pub(super) struct SfxState {
    pub(super) file: SfxFile,
    pub(super) samples: Vec<f32>,
    pub(super) pitch: Vec<f32>,
    rendered: Option<Sfx>,
    pub(super) history: Vec<SfxFile>,
    pub(super) name: String,
    pub(super) auto_play: bool,
    pub(super) started: Option<Instant>,
    pub(super) snippet: Option<String>,
    next_seed: u64,
}

impl Default for SfxState {
    fn default() -> Self {
        let mut state = Self {
            file: SfxFile {
                format: FORMAT,
                kind: Kind::Coin,
                seed: 1,
                sound: Kind::Coin.generate(1),
            },
            samples: Vec::new(),
            pitch: Vec::new(),
            rendered: None,
            history: Vec::new(),
            name: "coin".into(),
            auto_play: true,
            started: None,
            snippet: None,
            next_seed: 2,
        };
        state.refresh();
        state.remember();
        state
    }
}

impl SfxState {
    /// Re-renders only when the sound changed since the last render.
    pub(super) fn refresh(&mut self) {
        if self.rendered.as_ref() == Some(&self.file.sound) {
            return;
        }
        self.samples = self.file.sound.render();
        let seconds = self.file.sound.seconds();
        let pitch: Vec<f64> = (0..=96)
            .map(|i| self.file.sound.pitch(seconds * f64::from(i) / 96.0))
            .collect();
        // Pitch is drawn on a log axis from 40 Hz to 8 kHz.
        #[allow(clippy::cast_possible_truncation)]
        let normalized = pitch
            .iter()
            .map(|hz| ((hz / 40.0).log2() / 200_f64.log2()) as f32)
            .collect();
        self.pitch = normalized;
        self.rendered = Some(self.file.sound.clone());
    }

    fn remember(&mut self) {
        self.history.retain(|old| old != &self.file);
        self.history.insert(0, self.file.clone());
        self.history.truncate(HISTORY);
    }

    pub(super) fn generate(&mut self, kind: Kind) {
        let seed = self.next_seed;
        self.next_seed = self.next_seed.wrapping_add(1);
        self.file = SfxFile {
            format: FORMAT,
            kind,
            seed,
            sound: kind.generate(seed),
        };
        self.name = kind.slug().into();
        self.snippet = None;
        self.refresh();
        self.remember();
    }

    pub(super) fn mutate(&mut self) {
        let seed = self.next_seed;
        self.next_seed = self.next_seed.wrapping_add(1);
        self.file.sound = self.file.sound.mutate(seed);
        self.refresh();
        self.remember();
    }

    // Display only: the effect is at most a few seconds long.
    #[allow(clippy::cast_possible_truncation)]
    fn playhead(&self) -> Option<f32> {
        let elapsed = self.started?.elapsed().as_secs_f64();
        let at = elapsed / self.file.sound.seconds();
        (at <= 1.0).then_some(at as f32)
    }
}

impl ComposerApp {
    pub(super) fn sfx_workspace(&mut self, ui: &mut egui::Ui) {
        let wide = ui.available_width() >= 1100.0;
        egui::Panel::left("sfx-kinds")
            .default_size(236.0)
            .min_size(210.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.kind_picker(ui));
            });
        egui::Panel::right("sfx-params")
            .default_size(if wide { 340.0 } else { 300.0 })
            .min_size(280.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(8.0);
                    let changed = self.sfx_parameters(ui);
                    if changed.replay && self.sfx.auto_play {
                        self.audition();
                    }
                });
            });
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| self.sfx_stage(ui));
        });
        if self.sfx.playhead().is_some() {
            ui.ctx().request_repaint();
        }
    }

    /// The designer as one column: pick, hear, shape, take.
    pub(super) fn sfx_column(&mut self, ui: &mut egui::Ui) {
        self.kind_picker(ui);
        ui.add_space(6.0);
        self.sfx_stage(ui);
        ui.add_space(10.0);
        let changed = self.sfx_parameters(ui);
        if changed.replay && self.sfx.auto_play {
            self.audition();
        }
        ui.add_space(12.0);
        if self.sfx.playhead().is_some() {
            ui.ctx().request_repaint();
        }
    }

    fn kind_picker(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        theme::caption(ui, "GENERATE");
        ui.small("Each click is a new variation, played at once.");
        ui.add_space(4.0);
        let columns = if self.compact { 4 } else { 2 };
        let gaps = 6.0 * f32::from(u8::try_from(columns - 1).expect("columns"));
        let width =
            (ui.available_width() - gaps) / f32::from(u8::try_from(columns).expect("columns"));
        egui::Grid::new("kinds")
            .num_columns(2)
            .spacing(Vec2::new(6.0, 6.0))
            .show(ui, |ui| {
                for (i, kind) in Kind::ALL.into_iter().enumerate() {
                    let current = self.sfx.file.kind == kind;
                    let color = tint(kind);
                    let glyph: &str = icon(kind).into();
                    let mut text = egui::text::LayoutJob::default();
                    let format = |size: f32, color: Color32| egui::TextFormat {
                        font_id: egui::FontId::proportional(size),
                        color,
                        ..Default::default()
                    };
                    text.append(glyph, 0.0, format(22.0, color));
                    text.append(
                        &format!("\n{}", kind.name()),
                        0.0,
                        format(
                            if self.compact { 11.0 } else { 12.5 },
                            if current { color } else { theme::TEXT },
                        ),
                    );
                    text.halign = egui::Align::Center;
                    let tile = egui::Button::new(text)
                        .min_size(Vec2::new(width, 64.0))
                        .fill(if current {
                            color.gamma_multiply(0.16)
                        } else {
                            theme::RAISED
                        })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if current { color } else { theme::LINE_SOFT },
                        ))
                        .corner_radius(6);
                    if ui.add(tile).on_hover_text(kind.hint()).clicked() {
                        self.sfx.generate(kind);
                        self.audition();
                        self.status = format!("New {} · seed {}", kind.name(), self.sfx.file.seed);
                    }
                    if i % columns == columns - 1 {
                        ui.end_row();
                    }
                }
            });
        if self.compact {
            return;
        }
        ui.add_space(14.0);
        theme::caption(ui, "RECENT");
        let mut chosen = None;
        for (index, old) in self.sfx.history.iter().enumerate() {
            let current = *old == self.sfx.file;
            let glyph: &str = icon(old.kind).into();
            let row = egui::Button::new(
                RichText::new(format!("{glyph}  {} · {}", old.kind.name(), old.seed)).color(
                    if current {
                        tint(old.kind)
                    } else {
                        theme::MUTED
                    },
                ),
            )
            .frame(current)
            .fill(theme::WELL)
            .min_size(Vec2::new(ui.available_width(), 22.0));
            if ui.add(row).clicked() {
                chosen = Some(index);
            }
        }
        if let Some(index) = chosen {
            self.sfx.file = self.sfx.history[index].clone();
            self.sfx.name = self.sfx.file.kind.slug().into();
            self.sfx.refresh();
            self.audition();
        }
    }

    fn sfx_stage(&mut self, ui: &mut egui::Ui) {
        let kind = self.sfx.file.kind;
        let color = tint(kind);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(theme::label(icon(kind), kind.name()))
                    .size(22.0)
                    .color(color),
            );
            ui.label(
                RichText::new(format!(
                    "seed {}  ·  {:.2} s  ·  {}",
                    self.sfx.file.seed,
                    self.sfx.file.sound.seconds(),
                    self.sfx.file.sound.wave.name()
                ))
                .color(theme::FAINT),
            );
        });
        ui.small(kind.hint());
        ui.add_space(6.0);
        let playhead = self.sfx.playhead();
        let height = if self.compact {
            170.0
        } else {
            (ui.available_height() - 210.0).clamp(220.0, 420.0)
        };
        let wave = scope::waveform(
            ui,
            &self.sfx.samples,
            &self.sfx.pitch,
            height,
            color,
            playhead,
        );
        if wave
            .on_hover_text("Click to play · amber line is pitch")
            .clicked()
        {
            self.audition();
        }
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            let play = theme::primary(theme::label(icons::ICON_PLAY_ARROW, "Play"))
                .min_size(Vec2::new(96.0, 32.0));
            if ui.add(play).on_hover_text("Space").clicked() {
                self.audition();
            }
            if ui
                .add(
                    egui::Button::new(theme::label(icons::ICON_AUTO_FIX_HIGH, "Mutate"))
                        .min_size(Vec2::new(0.0, 32.0)),
                )
                .on_hover_text("A close variation of this sound (M)")
                .clicked()
            {
                self.sfx.mutate();
                self.audition();
            }
            if ui
                .add(
                    egui::Button::new(theme::label(icons::ICON_CASINO, "New variation"))
                        .min_size(Vec2::new(0.0, 32.0)),
                )
                .on_hover_text("Another sound of the same kind (R)")
                .clicked()
            {
                self.sfx.generate(kind);
                self.audition();
            }
            ui.checkbox(&mut self.sfx.auto_play, "Play on change");
        });
        ui.add_space(12.0);
        self.sfx_delivery(ui);
    }
}
