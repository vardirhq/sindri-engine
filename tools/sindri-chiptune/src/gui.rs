//! Standalone desktop controls. No Sindri editor integration.
use super::{Mood, Settings, compose_with, render_with};
use eframe::egui;
use std::{
    error::Error,
    process::Command,
    sync::mpsc::{self, Receiver},
    thread,
};

#[derive(Default)]
struct ComposerApp {
    settings: ComposerSettings,
    job: Option<Receiver<Result<String, String>>>,
    status: String,
}

struct ComposerSettings {
    seed: u64,
    bpm: f64,
    key: i32,
    energy: f64,
    mood: Mood,
    output: String,
}

impl Default for ComposerSettings {
    fn default() -> Self {
        Self {
            seed: 42,
            bpm: 118.0,
            key: 2,
            energy: 0.45,
            mood: Mood::Mysterious,
            output: "chiptune.wav".into(),
        }
    }
}

impl ComposerApp {
    fn generate(&mut self) {
        if self.job.is_some() {
            return;
        }
        let values = Settings {
            seed: self.settings.seed,
            bpm: self.settings.bpm,
            key: self.settings.key,
            energy: self.settings.energy,
            mood: self.settings.mood,
            output: self.settings.output.clone().into(),
        };
        let (tx, rx) = mpsc::channel();
        self.job = Some(rx);
        self.status = "Generating WAV...".into();
        thread::spawn(move || {
            let tracks = compose_with(&values);
            let result = render_with(&values, &tracks)
                .map(|()| format!("Saved {}", values.output.display()))
                .map_err(|error| error.to_string());
            let _ = tx.send(result);
        });
    }

    fn open_audio(&mut self) {
        let path = std::path::Path::new(&self.settings.output);
        if !path.is_file() {
            self.status = "Generate a WAV before opening it.".into();
            return;
        }
        #[cfg(target_os = "linux")]
        let result = Command::new("xdg-open").arg(path).spawn();
        #[cfg(target_os = "macos")]
        let result = Command::new("open").arg(path).spawn();
        #[cfg(target_os = "windows")]
        let result = Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(path)
            .spawn();
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        let result: std::io::Result<std::process::Child> = Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "No system file opener available",
        ));
        self.status = match result {
            Ok(_) => "Opened WAV using system default application.".into(),
            Err(error) => format!("Could not open audio: {error}"),
        };
    }
}

impl eframe::App for ComposerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(receiver) = &self.job {
            match receiver.try_recv() {
                Ok(result) => {
                    self.status = result.unwrap_or_else(|error| format!("Render failed: {error}"));
                    self.job = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status = "Render worker disconnected.".into();
                    self.job = None;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(100))
                }
            }
        }
        ui.vertical(|ui| {
            ui.heading("Sindri Chiptune Lab");
            ui.label("Standalone procedural Game Boy-inspired composition experiment");
            ui.separator();
            egui::Grid::new("composition_settings").num_columns(2).spacing([16.0, 12.0]).show(ui, |ui| {
                ui.label("Mood");
                egui::ComboBox::from_id_salt("mood")
                    .selected_text(match self.settings.mood {
                        Mood::Mysterious => "Mysterious",
                        Mood::Hopeful => "Hopeful",
                        Mood::Tense => "Tense",
                        Mood::Melancholic => "Melancholic",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.settings.mood, Mood::Mysterious, "Mysterious");
                        ui.selectable_value(&mut self.settings.mood, Mood::Hopeful, "Hopeful");
                        ui.selectable_value(&mut self.settings.mood, Mood::Tense, "Tense");
                        ui.selectable_value(&mut self.settings.mood, Mood::Melancholic, "Melancholic");
                    });
                ui.end_row();
                ui.label("Key (pitch class)");
                egui::ComboBox::from_id_salt("key")
                    .selected_text(["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"][usize::try_from(self.settings.key).unwrap_or(2)])
                    .show_ui(ui, |ui| {
                        for (index, name) in ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"].iter().enumerate() {
                            ui.selectable_value(&mut self.settings.key, i32::try_from(index).expect("12 keys"), *name);
                        }
                    });
                ui.end_row();
                ui.label("Tempo");
                ui.add(egui::Slider::new(&mut self.settings.bpm, 60.0..=220.0).suffix(" BPM"));
                ui.end_row();
                ui.label("Energy");
                ui.add(egui::Slider::new(&mut self.settings.energy, 0.05..=1.0));
                ui.end_row();
                ui.label("Seed");
                ui.add(egui::DragValue::new(&mut self.settings.seed).speed(1));
                ui.end_row();
                ui.label("Output WAV");
                ui.text_edit_singleline(&mut self.settings.output);
                ui.end_row();
            });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(self.job.is_none() && !self.settings.output.trim().is_empty(), egui::Button::new("Generate WAV")).clicked() {
                    self.generate();
                }
                if ui.button("Open WAV").clicked() {
                    self.open_audio();
                }
                if ui.button("Next seed").clicked() {
                    self.settings.seed = self.settings.seed.wrapping_add(1);
                }
            });
            ui.add_space(8.0);
            ui.label(&self.status);
            ui.separator();
            ui.small("32 bars · 4 channels · offline WAV export. The existing CLI is still available with --seed and --output.");
        });
    }
}

pub(super) fn run() -> Result<(), Box<dyn Error>> {
    eframe::run_native(
        "Sindri Chiptune Lab",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::<ComposerApp>::default())),
    )?;
    Ok(())
}
