//! Standalone workstation: presentation is separate from composition and audio.
mod editing;
mod files;
mod inspector;
mod overview;
mod panels;
mod platform;
mod scope;
mod score;
mod sfx_params;
mod sfx_view;
mod shell;
mod theme;
mod transport;
use crate::lab::live::Transport;
use crate::lab::{Note, Settings, compose_with};
use eframe::egui::{self, Color32};
use platform::{Files, Speaker};
use std::{path::PathBuf, sync::mpsc::Receiver};

const ACCENT: Color32 = theme::FORGE;
/// Below this width the side panels fold into one scrolling column.
const COMPACT: f32 = 720.0;
const TRACK_COLORS: [Color32; 5] = theme::VOICES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Music,
    Sfx,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InspectorTab {
    Song,
    Track,
    Mixer,
}

type Job = Receiver<Result<String, String>>;

struct ScoreSnapshot {
    candidate: usize,
    notes: [Vec<Note>; 3],
    bpm: f64,
}

struct ComposerApp {
    mode: Mode,
    /// A phone-width window: one scrolling column, bigger touch targets.
    compact: bool,
    inspector_tab: InspectorTab,
    sfx: sfx_view::SfxState,
    project: Option<PathBuf>,
    song_path: Option<PathBuf>,
    song_name: String,
    song_snippet: Option<String>,
    settings: Settings,
    generated: Settings,
    scores: [[Vec<Note>; 3]; 2],
    /// Locks are per candidate and remain local to the current workstation session.
    track_locks: [[bool; 3]; 2],
    regeneration: u64,
    selected: usize,
    section: usize,
    edit_track: usize,
    roll_base: i32,
    selected_note: Option<usize>,
    note_length: usize,
    drag_origin: Option<(usize, Note, usize, i32)>,
    undo: Vec<ScoreSnapshot>,
    redo: Vec<ScoreSnapshot>,
    job: Option<Job>,
    speaker: Option<Speaker>,
    files: Files,
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
            compact: false,
            inspector_tab: InspectorTab::Track,
            sfx: sfx_view::SfxState::default(),
            // Started inside a project, the tool delivers there without asking.
            #[cfg(not(target_arch = "wasm32"))]
            project: std::env::current_dir()
                .ok()
                .and_then(|dir| crate::lab::project::find(&dir)),
            #[cfg(target_arch = "wasm32")]
            project: None,
            song_path: None,
            song_name: "theme".into(),
            song_snippet: None,
            scores: Self::candidates(&settings),
            track_locks: [[false; 3]; 2],
            regeneration: 0,
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
            job: None,
            speaker: None,
            files: Files::default(),
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
        let has_locks = self.track_locks.iter().flatten().any(|locked| *locked);
        let mut generated = self.settings.clone();
        if has_locks {
            // A lock belongs to the current harmonic context. Keep seed, key
            // and mood fixed while replacing only unlocked parts around it.
            generated.seed = self.generated.seed;
            generated.key = self.generated.key;
            generated.mood = self.generated.mood;
        }
        let old_scores = self.scores.clone();
        let mut scores = Self::candidates(&generated);
        for candidate in 0..2 {
            for track in 0..3 {
                if self.track_locks[candidate][track] {
                    scores[candidate][track].clone_from(&old_scores[candidate][track]);
                } else if has_locks {
                    let mut candidate_settings = generated.clone();
                    candidate_settings.seed = candidate_settings
                        .seed
                        .wrapping_add(u64::try_from(candidate).expect("candidate"));
                    self.regeneration = self.regeneration.wrapping_add(1);
                    scores[candidate][track] = crate::lab::composer::compose_track(
                        &candidate_settings,
                        track,
                        self.regeneration,
                    );
                }
            }
        }
        self.scores = scores;
        self.selected_note = None;
        self.generated = generated;
        self.selected = 0;
        if self.transport.is_some() {
            self.start_live();
        }
        self.status = if has_locks {
            "Generated around the current harmony; locked tracks were kept. Unlock all tracks to apply a new seed, key, or mood.".into()
        } else {
            "Generated two new candidates. Select A or B to compare.".into()
        };
    }

    fn regenerate_track(&mut self) {
        let candidate = self.selected;
        let track = self.edit_track;
        if self.track_locks[candidate][track] {
            self.status = "Unlock this track before regenerating it.".into();
            return;
        }
        self.save_undo();
        self.regeneration = self.regeneration.wrapping_add(1);
        let mut settings = self.generated.clone();
        settings.seed = settings
            .seed
            .wrapping_add(u64::try_from(candidate).expect("candidate"));
        self.scores[candidate][track] =
            crate::lab::composer::compose_track(&settings, track, self.regeneration);
        self.selected_note = None;
        self.fit_pitch_range();
        self.refresh_edited_audio();
        self.status = format!(
            "Regenerated {} for candidate {}. The other tracks were kept.",
            ["Bass", "Arpeggio", "Lead"][track],
            if candidate == 0 { "A" } else { "B" }
        );
    }

    fn vary_melody(&mut self) {
        let candidate = self.selected;
        if self.edit_track != 2 {
            self.status = "Select Lead to create a melody variation.".into();
            return;
        }
        if self.track_locks[candidate][2] {
            self.status = "Unlock Lead before creating a melody variation.".into();
            return;
        }
        self.regeneration = self.regeneration.wrapping_add(1);
        let settings = self.selected_settings();
        let varied = crate::lab::composer::vary_lead(
            &self.scores[candidate][2],
            &settings,
            self.regeneration,
        );
        if varied == self.scores[candidate][2] {
            self.status = "Add more than one Lead note before creating a variation.".into();
            return;
        }
        self.save_undo();
        self.scores[candidate][2] = varied;
        self.selected_note = None;
        self.fit_pitch_range();
        self.refresh_edited_audio();
        self.status =
            "Created a lead variation. Rhythm, phrasing, and the ending were kept.".into();
    }
}

impl ComposerApp {
    fn workspace(&mut self, ui: &mut egui::Ui) {
        self.compact = ui.available_width() < COMPACT;
        if self.compact {
            // Fingers, not a pointer: taller rows and wider buttons.
            let spacing = &mut ui.style_mut().spacing;
            spacing.interact_size.y = 34.0;
            spacing.button_padding = egui::vec2(12.0, 7.0);
            spacing.item_spacing = egui::vec2(8.0, 8.0);
            spacing.slider_width = 130.0;
        }
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
        match (self.mode, self.compact) {
            (Mode::Music, false) => self.music_workspace(ui),
            (Mode::Sfx, false) => self.sfx_workspace(ui),
            (mode, true) => {
                egui::CentralPanel::default().show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| match mode {
                        Mode::Music => self.music_column(ui),
                        Mode::Sfx => self.sfx_column(ui),
                    });
                });
            }
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
        egui::Panel::left("composer")
            .default_size(292.0)
            .min_size(230.0)
            .resizable(true)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.inspector(ui);
                });
            });
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.score(ui);
            });
        });
    }
}

impl ComposerApp {
    /// The music workstation as one column, in the order a phone is used.
    fn music_column(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        self.transport_controls(ui);
        ui.add_space(6.0);
        self.inspector(ui);
        ui.add_space(8.0);
        self.score(ui);
        ui.add_space(12.0);
    }
}

impl eframe::App for ComposerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.workspace(ui);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn run() -> Result<(), Box<dyn std::error::Error>> {
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

#[cfg(target_arch = "wasm32")]
pub(super) fn start_web() {
    // `?mode=sfx` opens on the effects designer, so a link can say what it is for.
    let effects = web_sys_search().contains("mode=sfx");
    platform::start(move |cc| {
        theme::install(&cc.egui_ctx);
        Box::new(ComposerApp {
            mode: if effects { Mode::Sfx } else { Mode::Music },
            ..ComposerApp::default()
        })
    });
}

#[cfg(target_arch = "wasm32")]
fn web_sys_search() -> String {
    platform::search()
}

#[cfg(test)]
mod tests;
