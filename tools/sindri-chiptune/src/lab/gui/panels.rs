//! Composition inspector, live mixer, persistent transport and song delivery.
use super::{ACCENT, ComposerApp, TRACK_COLORS, files::snippet_row, theme};
use crate::lab::{Mood, instruments::Preset, live::Command};
use eframe::egui::{self, RichText, Vec2};
use egui_material_icons::icons;

const KEYS: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

impl ComposerApp {
    pub(super) fn controls(&mut self, ui: &mut egui::Ui) {
        theme::card(ui, "COMPOSE", ACCENT, |ui| {
            ui.small("Shape the next song, then generate two candidates.");
            ui.add_space(4.0);
            theme::caption(ui, "MOOD");
            let width = (ui.available_width() - 6.0) / 2.0;
            egui::Grid::new("moods")
                .num_columns(2)
                .spacing(Vec2::new(6.0, 6.0))
                .show(ui, |ui| {
                    for (i, (mood, icon, name)) in [
                        (Mood::Mysterious, icons::ICON_NIGHTS_STAY, "Mysterious"),
                        (Mood::Hopeful, icons::ICON_WB_SUNNY, "Hopeful"),
                        (Mood::Tense, icons::ICON_BOLT, "Tense"),
                        (Mood::Melancholic, icons::ICON_WATER_DROP, "Melancholic"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let on = self.settings.mood == mood;
                        let tile = egui::Button::new(
                            RichText::new(theme::label(icon, name)).color(if on {
                                theme::FORGE_BRIGHT
                            } else {
                                theme::TEXT
                            }),
                        )
                        .min_size(Vec2::new(width, 34.0))
                        .fill(if on { theme::EMBER } else { theme::WELL })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if on { ACCENT } else { theme::LINE_SOFT },
                        ));
                        if ui.add(tile).clicked() {
                            self.settings.mood = mood;
                        }
                        if i % 2 == 1 {
                            ui.end_row();
                        }
                    }
                });
            ui.add_space(6.0);
            egui::Grid::new("compose-values")
                .num_columns(2)
                .spacing(Vec2::new(10.0, 8.0))
                .show(ui, |ui| {
                    ui.label(RichText::new("Key").color(theme::MUTED));
                    egui::ComboBox::from_id_salt("key")
                        .selected_text(KEYS[usize::try_from(self.settings.key).unwrap_or(2)])
                        .width(70.0)
                        .show_ui(ui, |ui| {
                            for (i, name) in KEYS.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.settings.key,
                                    i32::try_from(i).expect("key"),
                                    *name,
                                );
                            }
                        });
                    ui.end_row();
                    ui.label(RichText::new("Density").color(theme::MUTED));
                    ui.add(
                        egui::Slider::new(&mut self.settings.energy, 0.05..=1.0).max_decimals(2),
                    );
                    ui.end_row();
                    ui.label(RichText::new("Seed").color(theme::MUTED));
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.settings.seed).speed(1));
                        if ui
                            .small_button(theme::label(icons::ICON_CASINO, ""))
                            .on_hover_text("Next seed")
                            .clicked()
                        {
                            self.settings.seed = self.settings.seed.wrapping_add(2);
                        }
                    });
                    ui.end_row();
                });
            ui.add_space(8.0);
            let generate = theme::primary(theme::label(icons::ICON_AUTO_AWESOME, "Generate A + B"))
                .min_size(Vec2::new(ui.available_width(), 32.0));
            if ui.add(generate).clicked() {
                self.generate();
            }
            ui.small("Mood, key, density and seed apply on Generate; the playing song never changes under you.");
        });
    }

    pub(super) fn instrument_rack(&mut self, ui: &mut egui::Ui) {
        theme::card(ui, "MIXER", ACCENT, |ui| {
            ui.small("Sounds and levels change live; notes stay intact.");
            ui.horizontal(|ui| {
                ui.label(RichText::new("Room & echo").color(theme::MUTED));
                ui.add(egui::Slider::new(&mut self.settings.rack.space, 0.0..=1.0).max_decimals(2))
                    .on_hover_text("How much reverb and echo the whole mix has");
            });
            ui.small("Chords are read from the bass line, so they follow your edits.");
            for (index, title) in ["BASS", "ARPEGGIO", "LEAD", "DRUMS", "CHORDS"]
                .iter()
                .enumerate()
            {
                let color = TRACK_COLORS[index];
                let rack = &mut self.settings.rack;
                let track = if index == 4 {
                    &mut rack.pad
                } else {
                    &mut rack.tracks[index]
                };
                ui.add_space(4.0);
                egui::Frame::new()
                    .fill(theme::WELL)
                    .corner_radius(5)
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let (dot, _) =
                                ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                            ui.painter().circle_filled(dot.center(), 4.5, color);
                            ui.label(RichText::new(*title).strong().size(11.5).color(color));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    theme::toggle(ui, &mut track.solo, "S", theme::FORGE_BRIGHT);
                                    theme::toggle(ui, &mut track.muted, "M", theme::DANGER);
                                },
                            );
                        });
                        let presets: &[Preset] = if index == 3 {
                            &Preset::DRUMS
                        } else {
                            &Preset::TONAL
                        };
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt(("preset", index))
                                .selected_text(track.preset.name())
                                .width(132.0)
                                .show_ui(ui, |ui| {
                                    for &preset in presets {
                                        ui.selectable_value(
                                            &mut track.preset,
                                            preset,
                                            preset.name(),
                                        );
                                    }
                                });
                            if index < 3 {
                                ui.add(
                                    egui::DragValue::new(&mut track.octave)
                                        .range(-2..=2)
                                        .prefix("oct "),
                                );
                            }
                        });
                        ui.add(
                            egui::Slider::new(&mut track.volume, 0.0..=1.0)
                                .show_value(true)
                                .max_decimals(2)
                                .text(RichText::new("level").color(theme::FAINT)),
                        );
                    });
            }
        });
    }

    pub(super) fn transport_controls(&mut self, ui: &mut egui::Ui) {
        if self.compact {
            self.compact_transport(ui);
            return;
        }
        ui.horizontal_wrapped(|ui| {
            let playing = self.transport.is_some() && !self.paused;
            let play = theme::primary(theme::label(
                if playing {
                    icons::ICON_PAUSE
                } else {
                    icons::ICON_PLAY_ARROW
                },
                if playing { "Pause" } else { "Play" },
            ))
            .min_size(Vec2::new(92.0, 30.0));
            if ui.add(play).on_hover_text("Space").clicked() {
                self.toggle_play();
            }
            if ui
                .add_enabled(
                    self.transport.is_some(),
                    egui::Button::new(theme::label(icons::ICON_STOP, ""))
                        .min_size(Vec2::new(30.0, 30.0)),
                )
                .on_hover_text("Stop")
                .clicked()
            {
                self.stop();
            }
            if ui
                .add(
                    egui::Button::new(theme::label(icons::ICON_SKIP_PREVIOUS, ""))
                        .min_size(Vec2::new(30.0, 30.0)),
                )
                .on_hover_text("Back to bar 1")
                .clicked()
                && let Some(transport) = &self.transport
            {
                let _ = transport.tx.send(Command::Seek(0));
            }
            if theme::toggle(
                ui,
                &mut self.looping,
                &theme::label(icons::ICON_REPEAT, "Loop"),
                ACCENT,
            ) && let Some(transport) = &self.transport
            {
                let _ = transport.tx.send(Command::Loop(self.looping));
            }
            ui.add_space(10.0);
            ui.label(
                RichText::new(format!("{:05.2}", self.playhead_bar() + 1.0))
                    .monospace()
                    .size(20.0)
                    .color(if playing {
                        theme::FORGE_BRIGHT
                    } else {
                        theme::MUTED
                    }),
            );
            ui.label(RichText::new("/ 32 BARS").size(10.5).color(theme::FAINT));
            ui.add_space(10.0);
            ui.add(
                egui::DragValue::new(&mut self.settings.bpm)
                    .range(60.0..=220.0)
                    .max_decimals(0)
                    .suffix(" BPM"),
            );
            ui.add_space(14.0);
            self.candidates_and_history(ui);
        });
    }

    fn compact_transport(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            ui.spacing_mut().button_padding.x = 4.0;
            let playing = self.transport.is_some() && !self.paused;
            let glyph = if playing {
                icons::ICON_PAUSE
            } else {
                icons::ICON_PLAY_ARROW
            };
            if ui
                .add(theme::primary(theme::label(glyph, "")).min_size(Vec2::new(36.0, 32.0)))
                .clicked()
            {
                self.toggle_play();
            }
            if ui
                .add_enabled(
                    self.transport.is_some(),
                    egui::Button::new(theme::label(icons::ICON_STOP, "")),
                )
                .clicked()
            {
                self.stop();
            }
            if ui
                .button(theme::label(icons::ICON_SKIP_PREVIOUS, ""))
                .clicked()
                && let Some(transport) = &self.transport
            {
                let _ = transport.tx.send(Command::Seek(0));
            }
            if theme::toggle(
                ui,
                &mut self.looping,
                &theme::label(icons::ICON_REPEAT, ""),
                ACCENT,
            ) && let Some(transport) = &self.transport
            {
                let _ = transport.tx.send(Command::Loop(self.looping));
            }
            if ui.small_button("−").clicked() {
                self.settings.bpm = (self.settings.bpm - 1.0).max(60.0);
            }
            ui.add(
                egui::DragValue::new(&mut self.settings.bpm)
                    .range(60.0..=220.0)
                    .max_decimals(0)
                    .suffix(" BPM"),
            );
            if ui.small_button("+").clicked() {
                self.settings.bpm = (self.settings.bpm + 1.0).min(220.0);
            }
            for candidate in 0..2 {
                let seed = self
                    .generated
                    .seed
                    .wrapping_add(u64::try_from(candidate).expect("candidate"));
                let letter = if candidate == 0 { "A" } else { "B" };
                let label = if seed < 1000 {
                    format!("{letter}·{seed}")
                } else {
                    letter.into()
                };
                if ui
                    .add_sized(
                        Vec2::new(34.0, 30.0),
                        egui::Button::new(RichText::new(label).size(11.0))
                            .selected(self.selected == candidate),
                    )
                    .on_hover_text(format!("Candidate {letter} · seed {seed}"))
                    .clicked()
                {
                    self.choose_candidate(candidate);
                }
            }
            ui.menu_button(theme::label(icons::ICON_MORE_HORIZ, ""), |ui| {
                ui.label(format!("Bar {:05.2} / 32", self.playhead_bar() + 1.0));
                self.history_controls(ui);
            });
        });
    }

    fn choose_candidate(&mut self, candidate: usize) {
        if self.selected != candidate {
            self.selected = candidate;
            self.focus_score_window();
            self.selected_note = None;
            if self.transport.is_some() {
                self.start_live();
            }
        }
    }

    /// The A/B candidate switch, then undo and redo.
    fn candidates_and_history(&mut self, ui: &mut egui::Ui) {
        for candidate in 0..2 {
            let seed = self
                .generated
                .seed
                .wrapping_add(u64::try_from(candidate).expect("candidate"));
            let text = format!("{} · {seed}", if candidate == 0 { "A" } else { "B" });
            if theme::segment(ui, self.selected == candidate, text) {
                self.choose_candidate(candidate);
            }
        }
        self.history_controls(ui);
    }

    fn history_controls(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        if ui
            .add_enabled(
                !self.undo.is_empty(),
                egui::Button::new(theme::label(icons::ICON_UNDO, "")),
            )
            .on_hover_text("Undo (Ctrl+Z)")
            .clicked()
        {
            self.undo_edit();
        }
        if ui
            .add_enabled(
                !self.redo.is_empty(),
                egui::Button::new(theme::label(icons::ICON_REDO, "")),
            )
            .on_hover_text("Redo (Ctrl+Y)")
            .clicked()
        {
            self.redo_edit();
        }
    }

    pub(super) fn song_delivery(&mut self, ui: &mut egui::Ui) {
        theme::card(ui, "SONG FILE", ACCENT, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(theme::label(icons::ICON_FOLDER_OPEN, "Open"))
                    .clicked()
                {
                    self.ask_song(ui.ctx());
                }
                if ui
                    .button(theme::label(icons::ICON_SAVE, "Save"))
                    .on_hover_text("Ctrl+S · the editable .sindrisong")
                    .clicked()
                {
                    self.save_song(false);
                }
                if ui
                    .button(theme::label(icons::ICON_UPLOAD_FILE, "MIDI in"))
                    .clicked()
                {
                    self.ask_midi(ui.ctx());
                }
                if ui
                    .button(theme::label(icons::ICON_DOWNLOAD, "MIDI"))
                    .clicked()
                {
                    self.export_midi();
                }
                if ui
                    .add_enabled(
                        self.job.is_none(),
                        egui::Button::new(theme::label(icons::ICON_AUDIO_FILE, "WAV")),
                    )
                    .clicked()
                {
                    self.export_wav();
                }
            });
        });
        ui.add_space(8.0);
        theme::card(ui, "USE IT IN A GAME", theme::FORGE, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Name").color(theme::MUTED));
                ui.add(
                    egui::TextEdit::singleline(&mut self.song_name)
                        .desired_width(ui.available_width()),
                );
            });
            let send = theme::primary(theme::label(icons::ICON_SEND, Self::send_label()))
                .min_size(Vec2::new(ui.available_width(), 30.0));
            if ui.add(send).on_hover_text(self.project_hint()).clicked() {
                self.send_to_project();
            }
            snippet_row(ui, self.song_snippet.as_deref());
        });
    }
}
