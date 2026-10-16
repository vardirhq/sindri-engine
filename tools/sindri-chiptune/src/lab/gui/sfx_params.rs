//! The designer's parameter cards. Every control edits the sound in place.
use super::{ComposerApp, scope, sfx_view::tint, theme};
use crate::lab::sfx::{Sfx, Wave};
use eframe::egui::{self, RichText};
use std::ops::RangeInclusive;

/// Whether an edit finished this frame, so the sound should play again.
pub(super) struct Changed {
    pub(super) replay: bool,
}

struct Rows<'a> {
    ui: &'a mut egui::Ui,
    replay: bool,
    changed: bool,
}

impl Rows<'_> {
    fn slider(
        &mut self,
        name: &str,
        value: &mut f64,
        range: RangeInclusive<f64>,
        suffix: &str,
        logarithmic: bool,
    ) {
        self.ui.label(RichText::new(name).color(theme::MUTED));
        let response = self.ui.add(
            egui::Slider::new(value, range)
                .suffix(suffix)
                .logarithmic(logarithmic)
                .max_decimals(3),
        );
        self.ui.end_row();
        self.changed |= response.changed();
        // A drag replays once, when it ends; a click or typed value replays at once.
        self.replay |= response.drag_stopped() || response.changed() && !response.dragged();
    }
}

fn grid(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut Rows<'_>)) -> (bool, bool) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing(egui::vec2(10.0, 6.0))
        .min_col_width(74.0)
        .show(ui, |ui| {
            let mut rows = Rows {
                ui,
                replay: false,
                changed: false,
            };
            add(&mut rows);
            (rows.replay, rows.changed)
        })
        .inner
}

impl ComposerApp {
    pub(super) fn sfx_parameters(&mut self, ui: &mut egui::Ui) -> Changed {
        let accent = tint(self.sfx.file.kind);
        let mut sound: Sfx = self.sfx.file.sound.clone();
        let mut replay = sound_cards(ui, &mut sound, accent);
        replay |= motion_cards(ui, &mut sound, accent);
        if sound != self.sfx.file.sound {
            self.sfx.file.sound = sound.clamped();
            self.sfx.snippet = None;
            self.sfx.refresh();
        }
        Changed { replay }
    }
}

/// Wave and envelope: what the sound is made of and how its level moves.
fn sound_cards(ui: &mut egui::Ui, sound: &mut Sfx, accent: egui::Color32) -> bool {
    let mut replay = false;
    theme::card(ui, "WAVE", accent, |ui| {
        ui.horizontal_wrapped(|ui| {
            for wave in Wave::ALL {
                if theme::segment(ui, sound.wave == wave, wave.name()) && sound.wave != wave {
                    sound.wave = wave;
                    replay = true;
                }
            }
        });
        if sound.wave == Wave::Square {
            let (r, _) = grid(ui, "duty", |rows| {
                rows.slider("Duty", &mut sound.duty, 0.05..=0.5, "", false);
                rows.slider("Duty sweep", &mut sound.duty_sweep, -2.0..=2.0, "/s", false);
            });
            replay |= r;
        }
    });
    ui.add_space(6.0);
    theme::card(ui, "ENVELOPE", accent, |ui| {
        let seconds = sound.seconds();
        let levels: Vec<f32> = (0..=64)
            .map(|i| {
                // A display sketch of a bounded level.
                #[allow(clippy::cast_possible_truncation)]
                let level = sound.envelope(seconds * f64::from(i) / 64.0) as f32;
                level
            })
            .collect();
        scope::envelope(ui, &levels, accent);
        ui.add_space(4.0);
        let (r, _) = grid(ui, "envelope", |rows| {
            rows.slider("Attack", &mut sound.attack, 0.0..=1.0, " s", false);
            rows.slider("Sustain", &mut sound.sustain, 0.0..=1.5, " s", false);
            rows.slider("Punch", &mut sound.punch, 0.0..=1.0, "", false);
            rows.slider("Decay", &mut sound.decay, 0.0..=2.0, " s", false);
        });
        replay |= r;
    });
    ui.add_space(6.0);
    replay
}

/// Pitch, motion and tone: how the sound moves and how it is filtered.
fn motion_cards(ui: &mut egui::Ui, sound: &mut Sfx, accent: egui::Color32) -> bool {
    let mut replay = false;
    theme::card(ui, "PITCH", accent, |ui| {
        let (r, _) = grid(ui, "pitch", |rows| {
            rows.slider("Start", &mut sound.frequency, 20.0..=8_000.0, " Hz", true);
            rows.slider("Slide", &mut sound.slide, -24.0..=24.0, " oct/s", false);
            rows.slider("Curve", &mut sound.accel, -64.0..=64.0, "", false);
            rows.slider("Stop below", &mut sound.cutoff, 0.0..=4_000.0, " Hz", false);
        });
        replay |= r;
    });
    ui.add_space(6.0);
    theme::card(ui, "MOTION", accent, |ui| {
        let (r, _) = grid(ui, "motion", |rows| {
            rows.slider(
                "Vibrato",
                &mut sound.vibrato_depth,
                0.0..=12.0,
                " st",
                false,
            );
            rows.slider(
                "Vib. rate",
                &mut sound.vibrato_rate,
                0.0..=40.0,
                " Hz",
                false,
            );
            rows.slider(
                "Arp jump",
                &mut sound.arp_semitones,
                -24.0..=24.0,
                " st",
                false,
            );
            rows.slider("Arp after", &mut sound.arp_delay, 0.0..=1.0, " s", false);
            rows.slider("Repeat", &mut sound.repeat, 0.0..=1.0, " s", false);
        });
        replay |= r;
    });
    ui.add_space(6.0);
    theme::card(ui, "TONE & OUTPUT", accent, |ui| {
        let (r, _) = grid(ui, "tone", |rows| {
            rows.slider(
                "Low-pass",
                &mut sound.low_pass,
                100.0..=20_000.0,
                " Hz",
                true,
            );
            rows.slider(
                "High-pass",
                &mut sound.high_pass,
                0.0..=4_000.0,
                " Hz",
                false,
            );
            rows.ui.label(RichText::new("Crush").color(theme::MUTED));
            let crush = rows
                .ui
                .add(egui::Slider::new(&mut sound.crush, 1..=16).suffix("×"));
            rows.ui.end_row();
            rows.replay |= crush.drag_stopped() || crush.changed() && !crush.dragged();
            rows.slider("Volume", &mut sound.volume, 0.0..=1.0, "", false);
        });
        replay |= r;
    });
    replay
}
