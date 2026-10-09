//! Standalone Chiptune Lab user interface. No Sindri editor dependencies.
use super::{Mood, RATE, Settings, compose_with, render_samples, render_with};
use eframe::egui::{self, Color32, RichText};
use rodio::{OutputStream, OutputStreamBuilder, Sink, buffer::SamplesBuffer};
use std::{error::Error, sync::mpsc::{self, Receiver}, thread};

enum JobResult { Audio(Vec<f32>), Saved(String) }
type Job = Receiver<Result<JobResult, String>>;

struct ComposerApp {
    settings: Settings,
    output: String,
    job: Option<Job>,
    stream: Option<OutputStream>,
    sink: Option<Sink>,
    status: String,
}
impl Default for ComposerApp {
    fn default() -> Self {
        Self {
            settings: Settings::default(), output: "chiptune.wav".into(),
            job: None, stream: None, sink: None,
            status: "Ready to compose. Preview plays in memory.".into(),
        }
    }
}
impl ComposerApp {
    fn start(&mut self, preview: bool) {
        if self.job.is_some() { return; }
        let mut settings = self.settings.clone();
        settings.output = self.output.clone().into();
        if self.sink.is_some() { self.stop(); }
        let (tx, rx) = mpsc::channel();
        self.job = Some(rx);
        self.status = if preview { "Composing preview..." } else { "Exporting WAV..." }.into();
        thread::spawn(move || {
            let notes = compose_with(&settings);
            let result = if preview {
                Ok(JobResult::Audio(render_samples(&settings, &notes)))
            } else {
                render_with(&settings, &notes)
                    .map(|()| JobResult::Saved(settings.output.display().to_string()))
                    .map_err(|error| error.to_string())
            };
            let _ = tx.send(result);
        });
    }
    fn stop(&mut self) {
        if let Some(sink) = self.sink.take() { sink.stop(); }
        self.stream = None;
        self.status = "Playback stopped.".into();
    }
    fn play(&mut self, samples: Vec<f32>) {
        match OutputStreamBuilder::open_default_stream() {
            Ok(stream) => {
                let sink = Sink::connect_new(stream.mixer());
                sink.append(SamplesBuffer::new(1, RATE, samples));
                sink.play();
                self.stream = Some(stream);
                self.sink = Some(sink);
                self.status = "Playing preview · no WAV file created.".into();
            }
            Err(error) => { self.status = format!("Audio device unavailable: {error}"); }
        }
    }
    fn poll(&mut self, ui: &egui::Ui) {
        let result = self.job.as_ref().map(Receiver::try_recv);
        match result {
            Some(Ok(Ok(JobResult::Audio(samples)))) => { self.job = None; self.play(samples); }
            Some(Ok(Ok(JobResult::Saved(path)))) => {
                self.job = None;
                self.status = format!("Export complete: {path}");
            }
            Some(Ok(Err(error))) => { self.job = None; self.status = format!("Error: {error}"); }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.job = None; self.status = "Render worker disconnected.".into();
            }
            Some(Err(mpsc::TryRecvError::Empty)) => {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(80));
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
            .selected_text(["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"][usize::try_from(self.settings.key).unwrap_or(2)])
            .show_ui(ui, |ui| {
                for (i, name) in ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"].iter().enumerate() {
                    ui.selectable_value(&mut self.settings.key, i32::try_from(i).expect("key"), *name);
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
    fn arrangement(&self, ui: &mut egui::Ui) {
        ui.heading("Arrangement");
        ui.label(RichText::new("32 BARS  /  4 SECTIONS  /  4 CHANNELS").color(Color32::from_gray(155)).small());
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            for (title, tint, description) in [
                ("A · INTRO", Color32::from_rgb(55, 105, 127), "Establish mood"),
                ("B · THEME", Color32::from_rgb(66, 143, 158), "Develop motif"),
                ("C · RISE", Color32::from_rgb(132, 94, 158), "Build tension"),
                ("D · OUTRO", Color32::from_rgb(62, 111, 126), "Release"),
            ] {
                egui::Frame::new().fill(tint.gamma_multiply(0.45)).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                    ui.set_min_width(130.0);
                    ui.label(RichText::new(title).strong().color(Color32::WHITE));
                    ui.label(RichText::new(description).color(Color32::WHITE).small());
                    ui.small("8 bars");
                });
            }
        });
        ui.add_space(16.0);
        ui.label("INSTRUMENTS");
        egui::Grid::new("channels").num_columns(2).spacing([20.0, 8.0]).show(ui, |ui| {
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
        ui.small("The preview is rendered to memory, not a temporary audio file. Export only when you like a variation.");
    }
}

impl eframe::App for ComposerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll(ui);
        ui.visuals_mut().panel_fill = Color32::from_rgb(20, 25, 34);
        ui.vertical(|ui| {
            ui.add_space(10.0);
            ui.label(RichText::new("SINDRI  /  SOUND LAB").color(Color32::from_rgb(105, 197, 211)).strong());
            ui.heading(RichText::new("Chiptune Composer").size(29.0));
            ui.label(RichText::new("A standalone procedural music playground").color(Color32::from_gray(170)));
            ui.add_space(18.0);
            ui.columns(2, |columns| {
                egui::Frame::group(columns[0].style()).show(&mut columns[0], |ui| self.controls(ui));
                egui::Frame::group(columns[1].style()).show(&mut columns[1], |ui| self.arrangement(ui));
            });
            ui.add_space(14.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.add_enabled(self.job.is_none(), egui::Button::new("▶  Preview")).clicked() {
                    self.start(true);
                }
                if ui.add_enabled(self.sink.is_some(), egui::Button::new("■  Stop")).clicked() {
                    self.stop();
                }
                if ui.add_enabled(self.job.is_none() && !self.output.trim().is_empty(), egui::Button::new("↓  Export WAV")).clicked() {
                    self.start(false);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Save as");
                ui.text_edit_singleline(&mut self.output);
            });
            ui.add_space(9.0);
            ui.label(RichText::new(&self.status).color(Color32::from_rgb(120, 200, 204)));
        });
    }
}

pub(super) fn run() -> Result<(), Box<dyn Error>> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([960.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native("Sindri Chiptune Lab", options, Box::new(|_| Ok(Box::<ComposerApp>::default())))?;
    Ok(())
}
