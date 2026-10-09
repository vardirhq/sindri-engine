//! Standalone Chiptune Lab user interface. No Sindri editor dependencies.
use super::instruments::Preset;
use super::live::{Command, LiveSource, Transport};
use super::{Mood, Settings, compose_with, render_with};
use eframe::egui::{self, Color32, RichText};
use rodio::{
    Player,
    stream::{DeviceSinkBuilder, MixerDeviceSink},
};
use std::{
    error::Error,
    sync::mpsc::{self, Receiver},
    thread,
};

enum JobResult {
    Saved(String),
}
type Job = Receiver<Result<JobResult, String>>;

struct ComposerApp {
    settings: Settings,
    output: String,
    job: Option<Job>,
    stream: Option<MixerDeviceSink>,
    sink: Option<Player>,
    transport: Option<Transport>,
    looping: bool,
    paused: bool,
    active_seed: u64,
    status: String,
}
impl Default for ComposerApp {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            output: "chiptune.wav".into(),
            job: None,
            stream: None,
            sink: None,
            transport: None,
            looping: true,
            paused: false,
            active_seed: 42,
            status: "Ready to compose. Preview plays in memory.".into(),
        }
    }
}
impl ComposerApp {
    fn export(&mut self, seed: u64) {
        if self.job.is_some() {
            return;
        }
        let mut settings = self.settings.clone();
        settings.seed = seed;
        settings.output = self.output.clone().into();
        let (tx, rx) = mpsc::channel();
        self.job = Some(rx);
        self.status = "Exporting WAV...".into();
        thread::spawn(move || {
            let notes = compose_with(&settings);
            let result = render_with(&settings, &notes)
                .map(|()| JobResult::Saved(settings.output.display().to_string()))
                .map_err(|error| error.to_string());
            let _ = tx.send(result);
        });
    }
    fn start_live(&mut self, seed: u64) {
        let mut settings = self.settings.clone();
        settings.seed = seed;
        self.active_seed = seed;
        let notes = compose_with(&settings);
        if let Some(transport) = &self.transport {
            let _ = transport.tx.send(Command::Replace(settings, notes));
            self.status = "Switched composition at the start of the song.".into();
            return;
        }
        match DeviceSinkBuilder::open_default_sink() {
            Ok(stream) => {
                let player = Player::connect_new(stream.mixer());
                let (source, transport) = LiveSource::new(settings, notes);
                let _ = transport.tx.send(Command::Loop(self.looping));
                player.append(source);
                player.play();
                self.stream = Some(stream);
                self.sink = Some(player);
                self.paused = false;
                self.transport = Some(transport);
                self.status = "Live playback. Change instruments or levels while listening.".into();
            }
            Err(error) => self.status = format!("Audio device unavailable: {error}"),
        }
    }
    fn stop(&mut self) {
        if let Some(player) = self.sink.take() {
            player.stop();
        }
        self.stream = None;
        self.transport = None;
        self.paused = false;
        self.status = "Playback stopped.".into();
    }
    fn sync_audio(&self, ui: &egui::Ui) {
        if let Some(transport) = &self.transport {
            let mut playing = self.settings.clone();
            playing.seed = self.active_seed;
            let _ = transport.tx.send(Command::Update(playing));
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(70));
        }
    }
    fn poll(&mut self, ui: &egui::Ui) {
        let result = self.job.as_ref().map(Receiver::try_recv);
        match result {
            Some(Ok(Ok(JobResult::Saved(path)))) => {
                self.job = None;
                self.status = format!("Export complete: {path}");
            }
            Some(Ok(Err(error))) => {
                self.job = None;
                self.status = format!("Error: {error}");
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.job = None;
                self.status = "Render worker disconnected.".into();
            }
            Some(Err(mpsc::TryRecvError::Empty)) => {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(80));
            }
            None => {}
        }
    }
    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Composition");
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
        ui.label("ENERGY / DENSITY");
        ui.add(egui::Slider::new(&mut self.settings.energy, 0.05..=1.0).show_value(true));
        ui.small("Low = spacious phrases. High = more rhythmic activity.");
        ui.add_space(10.0);
        ui.label("COMPOSITION SEED");
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.settings.seed).speed(1));
            if ui.button("New variation ↻").clicked() {
                self.settings.seed = self.settings.seed.wrapping_add(1);
                self.status = "Seed changed. Preview to hear the new composition.".into();
            }
        });
    }

    fn scorecard(&self, ui: &mut egui::Ui, seed: u64, title: &str, tint: Color32) {
        let mut settings = self.settings.clone();
        settings.seed = seed;
        let notes = compose_with(&settings);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).strong().color(tint));
                ui.label(
                    RichText::new(format!("SEED {seed}"))
                        .small()
                        .color(Color32::GRAY),
                );
            });
            let width = ui.available_width().max(120.0);
            let (area, _) = ui.allocate_exact_size(egui::vec2(width, 102.0), egui::Sense::hover());
            let painter = ui.painter_at(area);
            painter.rect_filled(area, 4.0, Color32::from_rgb(14, 21, 32));
            for beat in 0..=8 {
                let x = area.left()
                    + area.width() * f32::from(u16::try_from(beat).expect("beat")) / 8.0;
                painter.line_segment(
                    [egui::pos2(x, area.top()), egui::pos2(x, area.bottom())],
                    egui::Stroke::new(0.5, Color32::from_gray(48)),
                );
            }
            let colors = [
                Color32::from_rgb(81, 132, 168),
                Color32::from_rgb(146, 123, 185),
                Color32::from_rgb(108, 210, 187),
            ];
            for (channel, track) in notes.iter().enumerate() {
                for note in track.iter().filter(|note| note.start < 128) {
                    let x = area.left()
                        + area.width() * f32::from(u16::try_from(note.start).expect("start"))
                            / 128.0;
                    let w = (area.width() * f32::from(u16::try_from(note.len).expect("length"))
                        / 128.0)
                        .max(2.0);
                    let pitch =
                        f32::from(u16::try_from((note.pitch - 28).clamp(0, 70)).expect("pitch"));
                    let y = area.bottom() - 6.0 - pitch / 70.0 * 88.0;
                    painter.rect_filled(
                        egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, 3.0)),
                        1.0,
                        colors[channel],
                    );
                }
            }
            ui.label(
                RichText::new(format!(
                    "{} bass · {} arp · {} lead notes",
                    notes[0].len(),
                    notes[1].len(),
                    notes[2].len(),
                ))
                .small()
                .color(Color32::from_gray(151)),
            );
        });
    }

    fn compare(&self, ui: &mut egui::Ui) {
        ui.heading("Candidate comparison");
        ui.label(RichText::new("See the actual first eight bars of each composition. Audition either candidate before exporting.").color(Color32::from_gray(160)));
        ui.columns(2, |columns| {
            self.scorecard(
                &mut columns[0],
                self.settings.seed,
                "A  /  ORIGINAL",
                Color32::from_rgb(108, 210, 187),
            );
            self.scorecard(
                &mut columns[1],
                self.settings.seed.wrapping_add(1),
                "B  /  VARIATION",
                Color32::from_rgb(189, 145, 225),
            );
        });
    }
    fn transport_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .add_enabled(true, egui::Button::new("▶  Play / Switch A"))
                .clicked()
            {
                self.start_live(self.settings.seed);
            }
            if ui
                .add_enabled(true, egui::Button::new("▶  Switch B"))
                .clicked()
            {
                self.start_live(self.settings.seed.wrapping_add(1));
            }
            if ui
                .add_enabled(self.job.is_none(), egui::Button::new("Use B as A"))
                .clicked()
            {
                self.settings.seed = self.settings.seed.wrapping_add(1);
                self.status = "Candidate B is now A. Export uses the selected seed.".into();
            }
            if ui
                .add_enabled(self.sink.is_some(), egui::Button::new("■  Stop"))
                .clicked()
            {
                self.stop();
            }
            if ui
                .add_enabled(
                    self.job.is_none() && !self.output.trim().is_empty(),
                    egui::Button::new("↓  Export WAV"),
                )
                .clicked()
            {
                self.export(self.settings.seed);
            }
        });
        ui.horizontal(|ui| {
            if let Some(player) = &self.sink {
                if ui
                    .button(if self.paused {
                        "▶ Resume"
                    } else {
                        "Ⅱ Pause"
                    })
                    .clicked()
                {
                    if self.paused {
                        player.play();
                    } else {
                        player.pause();
                    }
                    self.paused = !self.paused;
                }
            }
            if ui.checkbox(&mut self.looping, "Loop").changed() {
                if let Some(transport) = &self.transport {
                    let _ = transport.tx.send(Command::Loop(self.looping));
                }
            }
            if let Some(transport) = &self.transport {
                let samples = transport
                    .position
                    .load(std::sync::atomic::Ordering::Relaxed);
                let bar = (samples as f64
                    / (f64::from(super::RATE) * 60.0 / self.settings.bpm * 4.0))
                    as usize;
                let mut bar_control = bar.min(31);
                if ui
                    .add(egui::Slider::new(&mut bar_control, 0..=31).text("Bar"))
                    .changed()
                {
                    let _ = transport.tx.send(Command::Seek(bar_control));
                }
                if ui.button("↤ Beginning").clicked() {
                    let _ = transport.tx.send(Command::Seek(0));
                }
                if ui.button("↦ Theme").clicked() {
                    let _ = transport.tx.send(Command::Seek(8));
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Save as");
            ui.text_edit_singleline(&mut self.output);
        });
    }
    fn instrument_rack(&mut self, ui: &mut egui::Ui) {
        ui.heading("Instrument rack");
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
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{}  {}", index + 1, title)).strong());
                    ui.separator();
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
                ui.horizontal(|ui| {
                    ui.label("Level");
                    ui.add(egui::Slider::new(&mut track.volume, 0.0..=1.0).show_value(true));
                    if index < 3 {
                        ui.label("Octave");
                        ui.add(egui::DragValue::new(&mut track.octave).range(-2..=2));
                    }
                });
            });
        }
        ui.small(
            "Changes affect the next preview or export. They never regenerate the note patterns.",
        );
    }

    fn arrangement(&self, ui: &mut egui::Ui) {
        ui.heading("Arrangement");
        ui.label(
            RichText::new("32 BARS  /  4 SECTIONS  /  4 CHANNELS")
                .color(Color32::from_gray(155))
                .small(),
        );
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            for (title, tint, description) in [
                (
                    "A · INTRO",
                    Color32::from_rgb(55, 105, 127),
                    "Establish mood",
                ),
                (
                    "B · THEME",
                    Color32::from_rgb(66, 143, 158),
                    "Develop motif",
                ),
                ("C · RISE", Color32::from_rgb(132, 94, 158), "Build tension"),
                ("D · OUTRO", Color32::from_rgb(62, 111, 126), "Release"),
            ] {
                egui::Frame::new()
                    .fill(tint.gamma_multiply(0.45))
                    .corner_radius(6.0)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.set_min_width(130.0);
                        ui.label(RichText::new(title).strong().color(Color32::WHITE));
                        ui.label(RichText::new(description).color(Color32::WHITE).small());
                        ui.small("8 bars");
                    });
            }
        });
        ui.add_space(16.0);
        ui.label("INSTRUMENTS");
        egui::Grid::new("channels")
            .num_columns(2)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                for (name, description) in [
                    ("01  WAVE BASS", "Roots, fifths, octave accents"),
                    ("02  PULSE ARP", "Seeded arpeggio contour"),
                    ("03  PULSE LEAD", "Seeded motifs and rhythmic shapes"),
                    ("04  NOISE", "Sparse percussion"),
                ] {
                    ui.label(RichText::new(name).strong());
                    ui.label(description);
                    ui.end_row();
                }
            });
        ui.add_space(12.0);
        ui.small("The sequencer runs continuously while you tune instruments. Export when you are satisfied.");
    }
}

impl eframe::App for ComposerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll(ui);
        self.sync_audio(ui);
        ui.visuals_mut().panel_fill = Color32::from_rgb(14, 19, 28);
        ui.visuals_mut().window_fill = Color32::from_rgb(18, 26, 38);
        ui.visuals_mut().widgets.inactive.bg_fill = Color32::from_rgb(37, 49, 66);
        ui.visuals_mut().widgets.hovered.bg_fill = Color32::from_rgb(57, 80, 99);
        ui.visuals_mut().selection.bg_fill = Color32::from_rgb(39, 129, 145);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(10.0);
            ui.label(
                RichText::new("SINDRI  /  SOUND LAB")
                    .color(Color32::from_rgb(105, 197, 211))
                    .strong(),
            );
            ui.heading(RichText::new("Chiptune Composer").size(29.0));
            ui.label(
                RichText::new("A standalone procedural music playground")
                    .color(Color32::from_gray(170)),
            );
            ui.add_space(16.0);
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(20, 35, 49))
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("LIVE TRANSPORT")
                            .strong()
                            .color(Color32::from_rgb(114, 206, 203)),
                    );
                    self.transport_controls(ui);
                    ui.label(RichText::new(&self.status).color(Color32::from_rgb(120, 200, 204)));
                });
            ui.add_space(16.0);
            ui.label(
                RichText::new("01   COMPOSITION & ARRANGEMENT")
                    .strong()
                    .color(Color32::from_rgb(114, 206, 203)),
            );
            ui.add_space(6.0);
            ui.columns(2, |columns| {
                egui::Frame::group(columns[0].style())
                    .show(&mut columns[0], |ui| self.controls(ui));
                egui::Frame::group(columns[1].style())
                    .show(&mut columns[1], |ui| self.arrangement(ui));
            });
            ui.add_space(14.0);
            ui.label(
                RichText::new("02   A / B COMPOSITION TIMELINE")
                    .strong()
                    .color(Color32::from_rgb(114, 206, 203)),
            );
            self.compare(ui);
            ui.add_space(14.0);
            ui.label(
                RichText::new("03   LIVE FOUR-CHANNEL MIXER")
                    .strong()
                    .color(Color32::from_rgb(114, 206, 203)),
            );
            self.instrument_rack(ui);
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(9.0);
        });
    }
}

pub(super) fn run() -> Result<(), Box<dyn Error>> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1160.0, 920.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Sindri Chiptune Lab",
        options,
        Box::new(|_| Ok(Box::<ComposerApp>::default())),
    )?;
    Ok(())
}
