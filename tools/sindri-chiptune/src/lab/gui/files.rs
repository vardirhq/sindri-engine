//! Opening, saving, exporting, and delivering sounds into a Sindri project.
use super::{
    ComposerApp, Mode,
    platform::{Opened, Purpose, SaveAs},
    theme,
};
use crate::lab::{midi, project, render_samples, sfx::SfxFile, song::SongFile, wav};
use eframe::egui::{self, RichText, Vec2};
use egui_material_icons::icons;

/// The words on the delivery button: a project folder natively, a download in a browser.
const SEND: &str = if cfg!(target_arch = "wasm32") {
    "Download"
} else {
    "Send to project"
};

fn text(file: &Opened) -> Result<&str, String> {
    std::str::from_utf8(&file.bytes).map_err(|_| "not a text file".into())
}

impl ComposerApp {
    #[cfg(not(target_arch = "wasm32"))]
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

    fn song_stem(&self) -> String {
        project::stem(&self.song_name).unwrap_or_else(|| "song".into())
    }

    /// Saves the song; natively a second save goes to the same file unasked.
    pub(super) fn save_song(&mut self, choose: bool) {
        let json = self.song_file().to_json();
        #[cfg(not(target_arch = "wasm32"))]
        if let (Some(path), false) = (&self.song_path, choose) {
            self.status = match std::fs::write(path, &json) {
                Ok(()) => format!("Saved {}", path.display()),
                Err(error) => format!("Save failed: {error}"),
            };
            return;
        }
        let _ = choose;
        let stem = self.song_stem();
        let save = SaveAs {
            title: "Save song",
            stem: &stem,
            extension: "sindrisong",
            label: "Sindri song",
        };
        match self.files.save(&save, json.as_bytes()) {
            Ok(Some(saved)) => {
                self.status = format!("Saved {saved}");
                self.song_path = Some(saved.into());
            }
            Ok(None) => {}
            Err(error) => self.status = format!("Save failed: {error}"),
        }
    }

    pub(super) fn ask_song(&mut self, context: &egui::Context) {
        self.files
            .open(context, Purpose::Song, "Sindri song", &["sindrisong"]);
    }

    pub(super) fn ask_midi(&mut self, context: &egui::Context) {
        self.files
            .open(context, Purpose::Midi, "MIDI", &["mid", "midi"]);
    }

    pub(super) fn ask_sfx(&mut self, context: &egui::Context) {
        self.files
            .open(context, Purpose::Sfx, "Sindri sound effect", &["sindrisfx"]);
    }

    pub(super) fn open_song(&mut self, file: &Opened) {
        match text(file).and_then(SongFile::from_json) {
            Ok(song) => {
                self.load_song(song);
                self.song_path = Some(file.name.clone().into());
                self.status = format!("Opened {}", file.name);
            }
            Err(error) => self.status = format!("Could not open {}: {error}", file.name),
        }
    }

    /// Puts a saved song in candidate A, with its own seed, key and mixer.
    pub(super) fn load_song(&mut self, song: SongFile) {
        let settings = song.settings();
        self.settings = settings.clone();
        self.generated = settings.clone();
        self.scores = Self::candidates(&settings);
        self.scores[0] = song.tracks;
        self.track_locks = [[false; 3]; 2];
        self.selected = 0;
        self.selected_note = None;
        self.undo.clear();
        self.redo.clear();
        self.fit_pitch_range();
        if self.transport.is_some() {
            self.start_live();
        }
        self.status = "Song loaded.".into();
    }

    pub(super) fn import_midi(&mut self, file: &Opened) {
        match midi::read(&file.bytes) {
            Ok((notes, bpm)) => {
                self.save_undo();
                self.scores[self.selected] = notes;
                self.track_locks[self.selected] = [false; 3];
                self.settings.bpm = bpm;
                self.selected_note = None;
                self.fit_pitch_range();
                self.refresh_edited_audio();
                self.status = "Imported MIDI, snapped to 1/16 steps.".into();
            }
            Err(error) => self.status = format!("MIDI import failed: {error}"),
        }
    }

    pub(super) fn open_sfx(&mut self, file: &Opened) {
        match text(file).and_then(SfxFile::from_json) {
            Ok(sound) => {
                let stem = std::path::Path::new(&file.name)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned());
                self.sfx.name = stem.unwrap_or_else(|| sound.kind.slug().into());
                self.sfx.file = sound;
                self.sfx.refresh();
                self.status = format!("Opened {}", file.name);
            }
            Err(error) => self.status = format!("Could not open {}: {error}", file.name),
        }
    }

    /// Renders the selected song to WAV: natively on a worker, in a browser now.
    pub(super) fn export_wav(&mut self) {
        let stem = self.song_stem();
        let save = SaveAs {
            title: "Export WAV",
            stem: &stem,
            extension: "wav",
            label: "WAV audio",
        };
        let settings = self.selected_settings();
        let notes = self.scores[self.selected].clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(path) = super::platform::Files::choose(&save) else {
                return;
            };
            let (tx, rx) = std::sync::mpsc::channel();
            self.job = Some(rx);
            self.status = "Exporting selected score...".into();
            std::thread::spawn(move || {
                let mut settings = settings;
                settings.output = path;
                let result = crate::lab::render_with(&settings, &notes)
                    .map(|()| settings.output.display().to_string())
                    .map_err(|e| e.to_string());
                let _ = tx.send(result);
            });
        }
        #[cfg(target_arch = "wasm32")]
        {
            let result = wav::encode(&render_samples(&settings, &notes), 2)
                .map_err(|e| e.to_string())
                .and_then(|bytes| self.files.save(&save, &bytes));
            self.report(result);
        }
    }

    pub(super) fn export_midi(&mut self) {
        let stem = self.song_stem();
        let save = SaveAs {
            title: "Export MIDI",
            stem: &stem,
            extension: "mid",
            label: "MIDI",
        };
        let result = midi::encode(&self.selected_settings(), &self.scores[self.selected])
            .map_err(str::to_owned)
            .and_then(|bytes| self.files.save(&save, &bytes));
        self.report(result);
    }

    fn save_sfx(&mut self, as_wav: bool) {
        let stem = project::stem(&self.sfx.name).unwrap_or_else(|| "sound".into());
        let save = SaveAs {
            title: "Save sound",
            stem: &stem,
            extension: if as_wav { "wav" } else { "sindrisfx" },
            label: if as_wav {
                "WAV audio"
            } else {
                "Sindri sound effect"
            },
        };
        let bytes = if as_wav {
            wav::encode(&self.sfx.samples, 1).map_err(|e| e.to_string())
        } else {
            Ok(self.sfx.file.to_json().into_bytes())
        };
        let result = bytes.and_then(|bytes| self.files.save(&save, &bytes));
        self.report(result);
    }

    fn report(&mut self, result: Result<Option<String>, String>) {
        match result {
            Ok(Some(saved)) => self.status = format!("Saved {saved}"),
            Ok(None) => {}
            Err(error) => self.status = format!("Save failed: {error}"),
        }
    }

    /// The current sound as a WAV and its editable source, named for a project.
    fn deliverable(
        &mut self,
    ) -> Result<(String, Vec<u8>, &'static str, &'static str, String), String> {
        if self.mode == Mode::Music {
            let song = self.song_file();
            let samples = render_samples(&song.settings(), &song.tracks);
            // Compressed natively, where the encoder builds; a browser download
            // stays WAV.
            #[cfg(not(target_arch = "wasm32"))]
            let (bytes, format) = (
                super::super::ogg::encode(&samples, 2).map_err(|e| e.to_string())?,
                "ogg",
            );
            #[cfg(target_arch = "wasm32")]
            let (bytes, format) = (wav::encode(&samples, 2).map_err(|e| e.to_string())?, "wav");
            Ok((
                self.song_name.clone(),
                bytes,
                format,
                "sindrisong",
                song.to_json(),
            ))
        } else {
            self.sfx.refresh();
            let bytes = wav::encode(&self.sfx.samples, 1).map_err(|e| e.to_string())?;
            Ok((
                self.sfx.name.clone(),
                bytes,
                "wav",
                "sindrisfx",
                self.sfx.file.to_json(),
            ))
        }
    }

    /// Writes the current sound into the chosen project, listed and ready to play.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn send_to_project(&mut self) {
        let Some(root) = self.project.clone() else {
            self.pick_project();
            if self.project.is_some() {
                self.send_to_project();
            }
            return;
        };
        let placed = self
            .deliverable()
            .and_then(|(name, bytes, format, extension, source)| {
                project::place(&root, &name, &bytes, format, extension, &source)
            });
        match placed {
            Ok(placed) => {
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
                self.delivered(&placed.clip);
            }
            Err(error) => self.status = format!("Could not send: {error}"),
        }
    }

    /// Downloads the WAV and its source, named as a project would hold them.
    #[cfg(target_arch = "wasm32")]
    pub(super) fn send_to_project(&mut self) {
        let downloaded = self
            .deliverable()
            .and_then(|(name, bytes, format, extension, source)| {
                let stem =
                    project::stem(&name).ok_or("Name the sound with letters or digits first.")?;
                let wav = SaveAs {
                    title: "",
                    stem: &stem,
                    extension: format,
                    label: "",
                };
                let editable = SaveAs {
                    title: "",
                    stem: &stem,
                    extension,
                    label: "",
                };
                self.files.save(&wav, &bytes)?;
                self.files.save(&editable, source.as_bytes())?;
                Ok(format!("audio/{stem}.{format}"))
            });
        match downloaded {
            Ok(clip) => {
                self.status = format!(
                    "Downloaded. Put both files in your project's assets/audio/ and add {}",
                    project::include_line(&clip)
                );
                self.delivered(&clip);
            }
            Err(error) => self.status = format!("Could not download: {error}"),
        }
    }

    fn delivered(&mut self, clip: &str) {
        let music = self.mode == Mode::Music;
        let snippet = project::snippet(clip, music);
        if music {
            self.song_snippet = Some(snippet);
        } else {
            self.sfx.snippet = Some(snippet);
        }
    }

    pub(super) fn sfx_delivery(&mut self, ui: &mut egui::Ui) {
        theme::card(ui, "USE IT IN A GAME", theme::FORGE, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Name").color(theme::MUTED));
                ui.add(egui::TextEdit::singleline(&mut self.sfx.name).desired_width(150.0));
                let send = theme::primary(theme::label(icons::ICON_SEND, SEND))
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
                    self.ask_sfx(ui.ctx());
                }
            });
            snippet_row(ui, self.sfx.snippet.as_deref());
        });
    }

    pub(super) fn send_label() -> &'static str {
        SEND
    }

    pub(super) fn project_hint(&self) -> String {
        if cfg!(target_arch = "wasm32") {
            return "Downloads the WAV and its editable source, named for assets/audio/".into();
        }
        self.project.as_ref().map_or_else(
            || "Choose a project folder first".into(),
            |root| {
                #[cfg(not(target_arch = "wasm32"))]
                let name = project::name(root);
                #[cfg(target_arch = "wasm32")]
                let name = root.display().to_string();
                format!(
                    "Writes assets/audio/<name>.wav and its source to {name}, and lists it in sindri.toml"
                )
            },
        )
    }
}

/// The Decay line for a delivered clip, with a copy button.
pub(super) fn snippet_row(ui: &mut egui::Ui, snippet: Option<&str>) {
    let Some(snippet) = snippet else {
        ui.small(if cfg!(target_arch = "wasm32") {
            "Download saves the WAV and its editable source. For a Sindri game, put both in assets/audio/; the line that plays it appears here."
        } else {
            "Send writes the WAV and its editable source into the project, lists the clip for export and gives you the line that plays it."
        });
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
