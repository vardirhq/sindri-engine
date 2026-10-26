//! Standalone workstation: presentation is separate from composition and audio.
mod development;
mod editing;
mod ensemble;
mod files;
mod inspector;
mod overview;
mod panels;
mod platform;
mod scope;
mod score;
mod seed;
mod sfx_params;
mod sfx_view;
mod shell;
mod theme;
mod tools;
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

#[derive(Clone, Copy)]
enum UndoKind {
    Notes,
    Sounds,
    Session,
}

struct ScoreSnapshot {
    candidate: usize,
    kind: UndoKind,
    scores: [[Vec<Note>; 3]; 2],
    settings: Settings,
    generated: Settings,
    racks: [crate::lab::instruments::Rack; 2],
    track_locks: [[bool; 3]; 2],
    sound_locks: [[bool; 5]; 2],
    arrangement_locks: [[bool; 5]; 2],
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
    seed_input: String,
    seed_error: Option<String>,
    generated: Settings,
    candidate_racks: [crate::lab::instruments::Rack; 2],
    sound_locks: [[bool; 5]; 2],
    arrangement_locks: [[bool; 5]; 2],
    ensemble_options: ensemble::Options,
    scores: [[Vec<Note>; 3]; 2],
    /// Locks are per candidate and remain local to the current workstation session.
    track_locks: [[bool; 3]; 2],
    regeneration: u64,
    selected: usize,
    section: usize,
    edit_track: usize,
    selected_layer: usize,
    arrangement: Option<overview::ArrangementPreview>,
    roll_span: usize,
    roll_page: usize,
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
    tools: tools::Tools,
    development: development::State,
}
impl Default for ComposerApp {
    fn default() -> Self {
        let mut settings = Settings::default();
        let candidate_racks = Self::ensemble_candidates(&settings);
        settings.rack = candidate_racks[0].clone();
        let mut app = Self {
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
            candidate_racks,
            sound_locks: [[false; 5]; 2],
            arrangement_locks: [[false; 5]; 2],
            ensemble_options: ensemble::Options::default(),
            seed_input: settings.seed.to_string(),
            seed_error: None,
            settings,
            selected: 0,
            section: 0,
            edit_track: 2,
            selected_layer: 2,
            arrangement: None,
            roll_span: 24,
            roll_page: 0,
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
            tools: tools::Tools::default(),
            development: development::State::default(),
        };
        app.refresh_arrangement();
        app.fit_pitch_range();
        app
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
        if self.selected != 0 {
            settings.seed_text = None;
        }
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
            generated.seed_text.clone_from(&self.generated.seed_text);
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
        self.generate_ensembles();
        generated.rack = self.settings.rack.clone();
        self.scores = scores;
        self.selected_note = None;
        self.generated = generated;
        self.selected = 0;
        self.focus_score_window();
        if self.transport.is_some() {
            self.start_live();
        }
        self.status = if has_locks {
            "Generated around the current note harmony; locked tracks were kept. Unlocked sounds and arrangements use the requested seed.".into()
        } else {
            "Generated two candidates with their own ensembles. Select A or B to compare.".into()
        };
    }

    fn regenerate_track(&mut self) {
        let candidate = self.selected;
        let track = self.edit_track;
        if self.selected_layer > 2 {
            self.status = "Select Bass, Arpeggio or Lead to regenerate notes.".into();
            return;
        }
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
        self.focus_score_window();
        self.refresh_edited_audio();
        self.status = format!(
            "Regenerated {} for candidate {}. The other tracks were kept.",
            ["Bass", "Arpeggio", "Lead"][track],
            if candidate == 0 { "A" } else { "B" }
        );
    }

    fn vary_melody(&mut self) {
        let candidate = self.selected;
        if self.selected_layer != 2 {
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
        let was_compact = self.compact;
        self.compact = ui.available_width() < COMPACT;
        if self.compact != was_compact {
            self.focus_score_window();
        }
        if self.compact {
            // Fingers, not a pointer: taller rows and wider buttons.
            let spacing = &mut ui.style_mut().spacing;
            spacing.interact_size.y = 30.0;
            spacing.button_padding = egui::vec2(7.0, 5.0);
            spacing.item_spacing = egui::vec2(8.0, 4.0);
            spacing.slider_width = 130.0;
        }
        self.refresh_arrangement();
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
            (Mode::Music, true) => self.compact_music_workspace(ui),
            (Mode::Sfx, true) => {
                egui::CentralPanel::default().show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| self.sfx_column(ui));
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
            .default_size(300.0)
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
    /// A resizable bottom inspector keeps sound controls available while editing.
    fn compact_music_workspace(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("track-sheet")
            .default_size(260.0)
            .min_size(110.0)
            .resizable(true)
            .frame(
                egui::Frame::new()
                    .fill(theme::HEADER)
                    .corner_radius(12)
                    .inner_margin(egui::Margin::same(10)),
            )
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.spacing_mut().interact_size.y = 26.0;
                ui.spacing_mut().button_padding.y = 3.0;
                egui::ScrollArea::vertical()
                    .id_salt("track-sheet-scroll")
                    .show(ui, |ui| self.inspector(ui));
            });
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("mobile-score-scroll")
                .show(ui, |ui| {
                    self.transport_controls(ui);
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
