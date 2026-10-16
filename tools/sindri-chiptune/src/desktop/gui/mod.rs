//! Standalone workstation: presentation is separate from composition and audio.
mod editing;
mod files;
mod overview;
mod panels;
mod scope;
mod score;
mod sfx_params;
mod sfx_view;
mod shell;
mod theme;
use crate::desktop::live::{Command, LiveSource, Transport};
use crate::desktop::{Note, Settings, compose_with, project, render_with};
use eframe::egui::{self, Color32};
use rodio::{
    Player,
    stream::{DeviceSinkBuilder, MixerDeviceSink},
};
use std::{
    error::Error,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
};

const ACCENT: Color32 = theme::FORGE;
const TRACK_COLORS: [Color32; 4] = theme::VOICES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Music,
    Sfx,
}
type Job = Receiver<Result<String, String>>;

struct ScoreSnapshot {
    candidate: usize,
    notes: [Vec<Note>; 3],
    bpm: f64,
}

struct ComposerApp {
    mode: Mode,
    sfx: sfx_view::SfxState,
    project: Option<PathBuf>,
    song_path: Option<PathBuf>,
    song_name: String,
    song_snippet: Option<String>,
    settings: Settings,
    generated: Settings,
    scores: [[Vec<Note>; 3]; 2],
    selected: usize,
    section: usize,
    edit_track: usize,
    roll_base: i32,
    selected_note: Option<usize>,
    note_length: usize,
    drag_origin: Option<(usize, Note, usize, i32)>,
    undo: Vec<ScoreSnapshot>,
    redo: Vec<ScoreSnapshot>,
    output: String,
    job: Option<Job>,
    stream: Option<MixerDeviceSink>,
    sink: Option<Player>,
    transport: Option<Transport>,
    playing: Option<Settings>,
    last_sent: Option<Settings>,
    looping: bool,
    paused: bool,
    status: String,
}
impl Default for ComposerApp {
    fn default() -> Self {
        let settings = Settings::default();
        Self {
            mode: Mode::Music,
            sfx: sfx_view::SfxState::default(),
            // Started inside a project, the tool delivers there without asking.
            project: std::env::current_dir()
                .ok()
                .and_then(|dir| project::find(&dir)),
            song_path: None,
            song_name: "theme".into(),
            song_snippet: None,
            scores: Self::candidates(&settings),
            generated: settings.clone(),
            settings,
            selected: 0,
            section: 0,
            edit_track: 2,
            roll_base: 48,
            selected_note: None,
            note_length: 2,
            drag_origin: None,
            undo: Vec::new(),
            redo: Vec::new(),
            output: "chiptune.wav".into(),
            job: None,
            stream: None,
            sink: None,
            transport: None,
            playing: None,
            last_sent: None,
            looping: true,
            paused: false,
            status: "Ready. Play a candidate, or switch to Sound FX for game effects.".into(),
        }
    }
}
impl ComposerApp {
    fn candidates(settings: &Settings) -> [[Vec<Note>; 3]; 2] {
        let mut b = settings.clone();
        b.seed = b.seed.wrapping_add(1);
        [compose_with(settings), compose_with(&b)]
    }
    fn selected_settings(&self) -> Settings {
        let mut settings = self.generated.clone();
        settings.seed = settings
            .seed
            .wrapping_add(u64::try_from(self.selected).expect("candidate"));
        settings.rack = self.settings.rack.clone();
        settings.bpm = self.settings.bpm;
        settings
    }
    fn generate(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.scores = Self::candidates(&self.settings);
        self.selected_note = None;
        self.generated = self.settings.clone();
        self.selected = 0;
        if self.transport.is_some() {
            self.start_live();
        }
        self.status = "Generated two new candidates. Select A or B to compare.".into();
    }
    fn export(&mut self) {
        if self.job.is_some() {
            return;
        }
        let mut settings = self.selected_settings();
        settings.output = self.output.clone().into();
        let notes = self.scores[self.selected].clone();
        let (tx, rx) = mpsc::channel();
        self.job = Some(rx);
        self.status = "Exporting selected score...".into();
        thread::spawn(move || {
            let result = render_with(&settings, &notes)
                .map(|()| settings.output.display().to_string())
                .map_err(|e| e.to_string());
            let _ = tx.send(result);
        });
    }
    fn start_live(&mut self) {
        let settings = self.selected_settings();
        let notes = self.scores[self.selected].clone();
        if let Some(transport) = &self.transport {
            let _ = transport.tx.send(Command::Replace(settings.clone(), notes));
            if let Some(player) = &self.sink {
                player.play();
            }
            self.paused = false;
        } else {
            match DeviceSinkBuilder::open_default_sink() {
                Ok(stream) => {
                    let player = Player::connect_new(stream.mixer());
                    let (source, transport) = LiveSource::new(settings.clone(), notes);
                    let _ = transport.tx.send(Command::Loop(self.looping));
                    player.append(source);
                    player.play();
                    self.stream = Some(stream);
                    self.sink = Some(player);
                    self.transport = Some(transport);
                    self.paused = false;
                }
                Err(error) => {
                    self.status = format!("Audio device unavailable: {error}");
                    return;
                }
            }
        }
        self.playing = Some(settings.clone());
        self.last_sent = Some(settings);
        self.status = "Playing selected score. Mixer and tempo changes are live.".into();
    }
    fn stop(&mut self) {
        if let Some(player) = self.sink.take() {
            player.stop();
        }
        self.stream = None;
        self.transport = None;
        self.playing = None;
        self.last_sent = None;
        self.paused = false;
        self.status = "Stopped.".into();
    }
    fn sync_audio(&mut self, ui: &egui::Ui) {
        if let (Some(transport), Some(playing)) = (&self.transport, &mut self.playing) {
            playing.rack = self.settings.rack.clone();
            playing.bpm = self.settings.bpm;
            if self.last_sent.as_ref() != Some(playing) {
                let _ = transport.tx.send(Command::Update(playing.clone()));
                self.last_sent = Some(playing.clone());
            }
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(33));
        }
    }
    fn playhead_bar(&self) -> f32 {
        self.transport.as_ref().map_or(0.0, |transport| {
            let samples = transport
                .position
                .load(std::sync::atomic::Ordering::Relaxed);
            let samples = u32::try_from(samples).unwrap_or(u32::MAX);
            // A 32-bar song has fewer than 6 million samples; display precision suffices.
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let bar = (f64::from(samples)
                / (f64::from(crate::desktop::RATE) * 240.0 / self.settings.bpm))
                as f32;
            bar.min(31.999)
        })
    }
    fn poll(&mut self, ui: &egui::Ui) {
        if self.sink.as_ref().is_some_and(Player::empty) && !self.paused {
            self.stop();
            self.status = "Finished. Press Play to audition again.".into();
        }
        match self.job.as_ref().map(Receiver::try_recv) {
            Some(Ok(result)) => {
                self.job = None;
                self.status = match result {
                    Ok(path) => format!("Saved {path}"),
                    Err(e) => format!("Export failed: {e}"),
                };
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.job = None;
                self.status = "Export worker disconnected.".into();
            }
            Some(Err(mpsc::TryRecvError::Empty)) => ui
                .ctx()
                .request_repaint_after(std::time::Duration::from_millis(80)),
            None => {}
        }
    }
}

impl ComposerApp {
    fn toggle_play(&mut self) {
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

    fn workspace(&mut self, ui: &mut egui::Ui) {
        self.poll(ui);
        self.shortcuts(ui);
        egui::Panel::top("brand")
            .exact_size(46.0)
            .frame(
                egui::Frame::new()
                    .fill(theme::HEADER)
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .stroke(egui::Stroke::new(1.0, theme::LINE)),
            )
            .show(ui, |ui| self.top_bar(ui));
        egui::Panel::bottom("status")
            .exact_size(26.0)
            .frame(
                egui::Frame::new()
                    .fill(theme::HEADER)
                    .inner_margin(egui::Margin::symmetric(10, 3))
                    .stroke(egui::Stroke::new(1.0, theme::LINE)),
            )
            .show(ui, |ui| self.status_bar(ui));
        match self.mode {
            Mode::Music => self.music_workspace(ui),
            Mode::Sfx => self.sfx_workspace(ui),
        }
        self.sync_audio(ui);
    }

    fn music_workspace(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("transport")
            .frame(
                egui::Frame::new()
                    .fill(theme::PANEL)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .stroke(egui::Stroke::new(1.0, theme::LINE_SOFT)),
            )
            .show(ui, |ui| self.transport_controls(ui));
        let wide = ui.available_width() >= 1100.0;
        egui::Panel::left("composer")
            .default_size(262.0)
            .min_size(230.0)
            .resizable(true)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(8.0);
                    self.controls(ui);
                    if !wide {
                        ui.add_space(8.0);
                        self.instrument_rack(ui);
                    }
                    ui.add_space(8.0);
                    self.song_delivery(ui);
                });
            });
        if wide {
            egui::Panel::right("mixer")
                .default_size(300.0)
                .min_size(270.0)
                .resizable(true)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add_space(8.0);
                        self.instrument_rack(ui);
                    });
                });
        }
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.score(ui);
            });
        });
    }
}

impl eframe::App for ComposerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.workspace(ui);
    }
}

pub(super) fn run() -> Result<(), Box<dyn Error>> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Sindri Sound Lab")
            .with_inner_size([1440.0, 880.0])
            .with_min_inner_size([820.0, 580.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Sindri Sound Lab",
        options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            Ok(Box::<ComposerApp>::default())
        }),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests;
