//! Composition inspector, live mixer and persistent transport.
use super::{ACCENT, ComposerApp, TRACK_COLORS};
use crate::desktop::{Mood, instruments::Preset, live::Command};
use eframe::egui::{self, Color32, RichText};

impl ComposerApp {
    pub(super) fn controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Compose");
        ui.small("Shape the next song, then press Generate.");
        ui.add_space(6.0);
        ui.label("MOOD / HARMONY");
        egui::ComboBox::from_id_salt("mood")
            .selected_text(match self.settings.mood {
                Mood::Mysterious => "Mysterious / exploratory",
                Mood::Hopeful => "Hopeful / uplifting",
                Mood::Tense => "Tense / uneasy",
                Mood::Melancholic => "Melancholic / reflective",
            })
            .width(230.0)
            .show_ui(ui, |ui| {
                for (mood, name) in [
                    (Mood::Mysterious, "Mysterious / exploratory"),
                    (Mood::Hopeful, "Hopeful / uplifting"),
                    (Mood::Tense, "Tense / uneasy"),
                    (Mood::Melancholic, "Melancholic / reflective"),
                ] {
                    ui.selectable_value(&mut self.settings.mood, mood, name);
                }
            });
        ui.add_space(8.0);
        ui.label("KEY");
        egui::ComboBox::from_id_salt("key")
            .selected_text(
                [
                    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
                ][usize::try_from(self.settings.key).unwrap_or(2)],
            )
            .show_ui(ui, |ui| {
                for (i, name) in [
                    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
                ]
                .iter()
                .enumerate()
                {
                    ui.selectable_value(
                        &mut self.settings.key,
                        i32::try_from(i).expect("key"),
                        *name,
                    );
                }
            });
        ui.add_space(10.0);
        ui.label("TEMPO");
        ui.add(egui::Slider::new(&mut self.settings.bpm, 60.0..=220.0).suffix(" BPM"));
        ui.add_space(6.0);
        ui.label("NOTE DENSITY");
        ui.add(egui::Slider::new(&mut self.settings.energy, 0.05..=1.0).show_value(true));
        ui.small("Low = spacious phrases. High = more rhythmic activity.");
        ui.add_space(10.0);
        ui.label("COMPOSITION SEED");
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.settings.seed).speed(1));
            if ui.button("New variation ↻").clicked() {
                self.settings.seed = self.settings.seed.wrapping_add(1);
                self.status = "Seed changed. Generate to hear the new composition.".into();
            }
        });
    }

    pub(super) fn instrument_rack(&mut self, ui: &mut egui::Ui) {
        ui.heading("Mixer");
        ui.label(
            RichText::new("Change sounds without changing the composition")
                .color(Color32::from_gray(160)),
        );
        ui.add_space(8.0);
        for (index, title) in ["BASS", "HARMONY / ARP", "LEAD", "PERCUSSION"]
            .iter()
            .enumerate()
        {
            let track = &mut self.settings.rack.tracks[index];
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.label(
                    RichText::new(format!("0{}  {}", index + 1, title))
                        .strong()
                        .color(TRACK_COLORS[index]),
                );
                ui.horizontal_wrapped(|ui| {
                    let presets: &[Preset] = if index == 3 {
                        &Preset::DRUMS
                    } else {
                        &Preset::TONAL
                    };
                    egui::ComboBox::from_id_salt(("preset", index))
                        .selected_text(track.preset.name())
                        .width(145.0)
                        .show_ui(ui, |ui| {
                            for &preset in presets {
                                ui.selectable_value(&mut track.preset, preset, preset.name());
                            }
                        });
                    ui.toggle_value(&mut track.muted, "Mute");
                    ui.toggle_value(&mut track.solo, "Solo");
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("Level");
                    ui.add(egui::Slider::new(&mut track.volume, 0.0..=1.0).show_value(true));
                    if index < 3 {
                        ui.label("Octave");
                        ui.add(egui::DragValue::new(&mut track.octave).range(-2..=2));
                    }
                });
            });
        }
        ui.small("Levels, sounds and mute/solo update live. Notes stay intact.");
    }

    pub(super) fn transport_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("SINDRI / SOUND LAB").strong().color(ACCENT));
            ui.separator();
            if ui
                .add(
                    egui::Button::new(if self.transport.is_some() && !self.paused {
                        "Pause"
                    } else {
                        "Play"
                    })
                    .fill(ACCENT.gamma_multiply(0.35)),
                )
                .clicked()
            {
                if let Some(player) = &self.sink {
                    if self.paused {
                        player.play();
                    } else {
                        player.pause();
                    }
                    self.paused = !self.paused;
                } else {
                    self.start_live();
                }
            }
            if ui
                .add_enabled(self.sink.is_some(), egui::Button::new("Stop"))
                .clicked()
            {
                self.stop();
            }
            if ui.checkbox(&mut self.looping, "Loop").changed()
                && let Some(transport) = &self.transport
            {
                let _ = transport.tx.send(Command::Loop(self.looping));
            }
            ui.separator();
            ui.label("Tempo");
            ui.add(
                egui::DragValue::new(&mut self.settings.bpm)
                    .range(60.0..=220.0)
                    .suffix(" BPM"),
            );
            ui.label(
                RichText::new(format!("BAR {:02} / 32", self.playhead_bar() + 1.0)).monospace(),
            );
            if ui.button("Restart").clicked()
                && let Some(transport) = &self.transport
            {
                let _ = transport.tx.send(Command::Seek(0));
            }
        });
    }

    pub(super) fn export_controls(&mut self, ui: &mut egui::Ui) {
        self.midi_controls(ui);
        ui.horizontal_wrapped(|ui| {
            ui.label("WAV path");
            ui.add(egui::TextEdit::singleline(&mut self.output).desired_width(200.0));
            if ui
                .add_enabled(
                    self.job.is_none() && !self.output.trim().is_empty(),
                    egui::Button::new("Export selected WAV"),
                )
                .clicked()
            {
                self.export();
            }
            ui.separator();
            ui.label(&self.status);
        });
    }
}
