//! Opening, saving, exporting, and delivering sounds into a Sindri project.
use super::{ComposerApp, Mode, theme};
use crate::desktop::{midi, project, render_samples, sfx::SfxFile, song::SongFile, wav};
use eframe::egui::{self, RichText, Vec2};
use egui_material_icons::icons;
use std::{fs, path::PathBuf};

fn dialog(title: &str, name: &str, extension: &str, label: &str) -> rfd::FileDialog {
    rfd::FileDialog::new()
        .set_title(title)
        .set_file_name(format!("{name}.{extension}"))
        .add_filter(label, &[extension])
}

impl ComposerApp {
    pub(super) fn pick_project(&mut self) {
        let Some(folder) = rfd::FileDialog::new()
            .set_title("Choose a Sindri project")
            .pick_folder()
        else {
            return;
        };
        match project::find(&folder) {
            Some(root) => {
                self.status = format!("Sounds now go to {}.", project::name(&root));
                self.project = Some(root);
            }
            None => self.status = format!("{} has no sindri.toml.", folder.display()),
        }
    }

    pub(super) fn song_file(&self) -> SongFile {
        SongFile::new(&self.selected_settings(), &self.scores[self.selected])
    }

    pub(super) fn save_song(&mut self, choose: bool) {
        let path = match (&self.song_path, choose) {
            (Some(path), false) => Some(path.clone()),
            _ => dialog("Save song", "song", "sindrisong", "Sindri song").save_file(),
        };
        let Some(path) = path else { return };
        self.status = match fs::write(&path, self.song_file().to_json()) {
            Ok(()) => {
                self.song_path = Some(path.clone());
                format!("Saved {}", path.display())
            }
            Err(error) => format!("Save failed: {error}"),
        };
    }

    pub(super) fn open_song(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Sindri song", &["sindrisong"])
            .pick_file()
        else {
            return;
        };
        match fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|text| SongFile::from_json(&text))
        {
            Ok(song) => self.load_song(song, Some(path)),
            Err(error) => self.status = format!("Could not open {}: {error}", path.display()),
        }
    }

    /// Puts a saved song in candidate A, with its own seed, key and mixer.
    pub(super) fn load_song(&mut self, song: SongFile, path: Option<PathBuf>) {
        let settings = song.settings();
        self.settings = settings.clone();
        self.generated = settings.clone();
        self.scores = Self::candidates(&settings);
        self.scores[0] = song.tracks;
        self.selected = 0;
        self.selected_note = None;
        self.undo.clear();
        self.redo.clear();
        self.fit_pitch_range();
        if self.transport.is_some() {
            self.start_live();
        }
        self.status = path.as_ref().map_or_else(
            || "Song loaded.".into(),
            |p| format!("Opened {}", p.display()),
        );
        self.song_path = path;
    }

    pub(super) fn export_wav(&mut self) {
        let Some(path) = dialog("Export WAV", "song", "wav", "WAV audio").save_file() else {
            return;
        };
        self.output = path.display().to_string();
        self.export();
    }

    pub(super) fn export_midi(&mut self) {
        let Some(path) = dialog("Export MIDI", "song", "mid", "MIDI").save_file() else {
            return;
        };
        let written = midi::write(
            &path.display().to_string(),
            &self.selected_settings(),
            &self.scores[self.selected],
        );
        self.status = match written {
            Ok(()) => format!(
                "Saved {}: notes and tempo; chip sounds stay here.",
                path.display()
            ),
            Err(error) => format!("MIDI export failed: {error}"),
        };
    }

    pub(super) fn import_midi(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("MIDI", &["mid", "midi"])
            .pick_file()
        else {
            return;
        };
        match fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|data| midi::read(&data))
        {
            Ok((notes, bpm)) => {
                self.save_undo();
                self.scores[self.selected] = notes;
                self.settings.bpm = bpm;
                self.selected_note = None;
                self.fit_pitch_range();
                self.refresh_edited_audio();
                self.status = "Imported MIDI, snapped to 1/16 steps.".into();
            }
            Err(error) => self.status = format!("MIDI import failed: {error}"),
        }
    }

    pub(super) fn open_sfx(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Sindri sound effect", &["sindrisfx"])
            .pick_file()
        else {
            return;
        };
        match fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|text| SfxFile::from_json(&text))
        {
            Ok(file) => {
                self.sfx.name = path.file_stem().map_or_else(
                    || file.kind.slug().into(),
                    |s| s.to_string_lossy().into_owned(),
                );
                self.sfx.file = file;
                self.sfx.refresh();
                self.status = format!("Opened {}", path.display());
            }
            Err(error) => self.status = format!("Could not open {}: {error}", path.display()),
        }
    }

    fn save_sfx(&mut self, as_wav: bool) {
        let stem = project::stem(&self.sfx.name).unwrap_or_else(|| "sound".into());
        let (extension, label) = if as_wav {
            ("wav", "WAV audio")
        } else {
            ("sindrisfx", "Sindri sound effect")
        };
        let Some(path) = dialog("Save sound", &stem, extension, label).save_file() else {
            return;
        };
        let result = if as_wav {
            wav::write(&path, &self.sfx.samples).map_err(|e| e.to_string())
        } else {
            fs::write(&path, self.sfx.file.to_json()).map_err(|e| e.to_string())
        };
        self.status = match result {
            Ok(()) => format!("Saved {}", path.display()),
            Err(error) => format!("Save failed: {error}"),
        };
    }

    /// Writes the current sound into the chosen project, listed and ready to play.
    pub(super) fn send_to_project(&mut self) {
        let Some(root) = self.project.clone() else {
            self.pick_project();
            if self.project.is_some() {
                self.send_to_project();
            }
            return;
        };
        let music = self.mode == Mode::Music;
        let result = if music {
            let song = self.song_file();
            let samples = render_samples(&song.settings(), &song.tracks);
            wav::encode(&samples)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    project::place(
                        &root,
                        &self.song_name,
                        &bytes,
                        "sindrisong",
                        &song.to_json(),
                    )
                })
        } else {
            self.sfx.refresh();
            wav::encode(&self.sfx.samples)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    project::place(
                        &root,
                        &self.sfx.name,
                        &bytes,
                        "sindrisfx",
                        &self.sfx.file.to_json(),
                    )
                })
        };
        match result {
            Ok(placed) => {
                let snippet = project::snippet(&placed.clip, music);
                self.status = format!(
                    "Sent {} to {}{}",
                    placed.clip,
                    project::name(&root),
                    if placed.listed {
                        ", listed for export"
                    } else {
                        ""
                    }
                );
                if music {
                    self.song_snippet = Some(snippet);
                } else {
                    self.sfx.snippet = Some(snippet);
                }
            }
            Err(error) => self.status = format!("Could not send: {error}"),
        }
    }

    pub(super) fn sfx_delivery(&mut self, ui: &mut egui::Ui) {
        theme::card(ui, "USE IT IN A GAME", theme::FORGE, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Name").color(theme::MUTED));
                ui.add(egui::TextEdit::singleline(&mut self.sfx.name).desired_width(150.0));
                let send = theme::primary(theme::label(icons::ICON_SEND, "Send to project"))
                    .min_size(Vec2::new(0.0, 28.0));
                if ui.add(send).on_hover_text(self.project_hint()).clicked() {
                    self.send_to_project();
                }
                if ui
                    .button(theme::label(icons::ICON_DOWNLOAD, "WAV"))
                    .clicked()
                {
                    self.save_sfx(true);
                }
                if ui
                    .button(theme::label(icons::ICON_SAVE, "Save"))
                    .on_hover_text("Save the editable .sindrisfx")
                    .clicked()
                {
                    self.save_sfx(false);
                }
                if ui
                    .button(theme::label(icons::ICON_FOLDER_OPEN, "Open"))
                    .clicked()
                {
                    self.open_sfx();
                }
            });
            snippet_row(ui, self.sfx.snippet.as_deref());
        });
    }

    pub(super) fn project_hint(&self) -> String {
        self.project.as_ref().map_or_else(
            || "Choose a project folder first".into(),
            |root| {
                format!(
                    "Writes assets/audio/<name>.wav and its source to {}, and lists it in sindri.toml",
                    project::name(root)
                )
            },
        )
    }
}

/// The Decay line for a delivered clip, with a copy button.
pub(super) fn snippet_row(ui: &mut egui::Ui, snippet: Option<&str>) {
    let Some(snippet) = snippet else {
        ui.small("Send writes the WAV and its editable source into the project, lists the clip for export and gives you the line that plays it.");
        return;
    };
    ui.add_space(4.0);
    egui::Frame::new()
        .fill(theme::WELL)
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(snippet).monospace().color(theme::SUCCESS));
                if ui
                    .small_button(theme::label(icons::ICON_CONTENT_COPY, "Copy"))
                    .clicked()
                {
                    ui.ctx().copy_text(snippet.to_owned());
                }
            });
        });
}
