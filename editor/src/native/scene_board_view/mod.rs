//! The Scenes panel: the project's scenes as a board of cards, the doors
//! between them drawn as arrows, and the project's scene list edited in place.
//!
//! What the board says is [`crate::project::SceneBoard`]'s business. This is
//! where it is drawn, where a card is clicked open, and where a card's picture
//! comes from — the last frame either view drew of that scene.

mod cards;
mod pictures;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::project::{AssetKind, Project, SceneBoard};
use crate::ui::icons;
use crate::ui::theme::metric;
use crate::ui::widgets::panel;

use self::cards::{BoardContext, board_view};
pub(super) use self::pictures::ScenePictures;
use super::EditorApp;
use super::unsaved::Discarding;

/// How long a board is trusted before the files it was read from are read
/// again. Short enough that a door added in a script appears while the panel
/// is being looked at; long enough that nothing is read at frame rate.
const STALE_AFTER: Duration = Duration::from_secs(2);

/// What the board panel holds between frames.
#[derive(Default)]
pub(super) struct SceneBoardState {
    board: Option<SceneBoard>,
    read_at: Option<Instant>,
    pictures: ScenePictures,
}

impl SceneBoardState {
    /// Asks for the board to be read again on the next frame it is drawn,
    /// because something it was read from has just been written.
    pub(super) fn invalidate(&mut self) {
        self.read_at = None;
    }

    pub(super) const fn pictures_mut(&mut self) -> &mut ScenePictures {
        &mut self.pictures
    }
}

/// What a frame of the board asked for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum BoardAction {
    Open(PathBuf),
    SetMain(PathBuf),
    Add(PathBuf),
    Remove(PathBuf),
    /// Moves a listed scene to an index of the manifest's `scenes` list.
    Move(PathBuf, usize),
    ShowInProject(PathBuf),
}

impl EditorApp {
    /// The Scenes panel.
    pub(super) fn scene_board_body(&mut self, ui: &mut egui::Ui) {
        let context = ui.ctx().clone();
        let Some(root) = self.open_project_root.clone() else {
            panel::empty_state(
                ui,
                icons::SCENES,
                "No project is open",
                "A project's scenes, and the doors between them, are shown here once one is open.",
            );
            return;
        };
        let stale = self
            .scene_board
            .read_at
            .is_none_or(|at| at.elapsed() >= STALE_AFTER);
        if stale {
            self.scene_board.read_at = Some(Instant::now());
            match Project::open(&root) {
                Ok(project) => self.scene_board.board = Some(SceneBoard::read(&project)),
                Err(error) => {
                    self.scene_board.board = None;
                    panel::note(ui, &error.to_string());
                    return;
                }
            }
        }
        // Repainted while visible so a door written in a script, or a picture
        // taken in another view, shows without waiting for a pointer move.
        context.request_repaint_after(STALE_AFTER);
        let Some(board) = self.scene_board.board.clone() else {
            return;
        };
        let unlisted = self
            .project
            .entries()
            .iter()
            .filter(|entry| entry.kind == AssetKind::Scene)
            .filter(|entry| !board.cards.iter().any(|card| card.path == entry.path))
            .map(|entry| (entry.path.clone(), entry.relative.clone()))
            .collect();
        let space = self.board_room(ui.max_rect());
        let action = ui
            .scope_builder(egui::UiBuilder::new().max_rect(space), |ui| {
                board_view(
                    ui,
                    &board,
                    &self.scene_board.pictures,
                    &BoardContext {
                        open: self.file.path(),
                        unlisted,
                    },
                )
            })
            .inner;
        if let Some(action) = action {
            self.act_on_board(action, &context);
        }
    }

    /// The part of the panel nothing floats over.
    ///
    /// A docked board has the whole of its panel. In the canvas arrangement
    /// the centre runs under the title bar, the status bar and the overlays,
    /// which suits a world that carries on beneath them and does not suit a
    /// board whose first card would sit under the hierarchy. So the board
    /// keeps to the columns between the overlays on either side, unless that
    /// leaves no room for a card.
    fn board_room(&self, panel: egui::Rect) -> egui::Rect {
        if !self.preferences.workspace.chrome().floats() || !self.dock.drawing_main {
            return panel;
        }
        let field = self.unobscured(panel);
        let middle = field.center().x;
        let mut room = field;
        for (_, overlay) in &self.dock.overlays {
            if !overlay.intersects(field) {
                continue;
            }
            if overlay.center().x < middle {
                room.min.x = room.min.x.max(overlay.right() + metric::GUTTER);
            } else {
                room.max.x = room.max.x.min(overlay.left() - metric::GUTTER);
            }
        }
        if room.width() < cards::CARD_MIN_WIDTH {
            field
        } else {
            room
        }
    }

    fn act_on_board(&mut self, action: BoardAction, context: &egui::Context) {
        match action {
            BoardAction::Open(path) => {
                if self.file.path() != Some(path.as_path()) {
                    self.discard_or_confirm(Discarding::OpenPath(path), context);
                }
                // Into the scene, as stepping into a panel of a storyboard: the
                // board is where a scene is chosen, the Scene view where it is
                // worked on, and the first frame drawn there is its picture.
                self.preferences.workspace.reveal(crate::dock::Panel::Scene);
            }
            BoardAction::SetMain(path) => self.set_main_scene(&path),
            BoardAction::Add(path) => self.edit_scene_list(&path, "carries", Project::add_scene),
            BoardAction::Remove(path) => {
                self.edit_scene_list(&path, "no longer carries", Project::remove_scene);
            }
            BoardAction::Move(path, to) => {
                self.edit_scene_list(&path, "moved", |project, scene| {
                    project.move_scene(scene, to)
                });
            }
            BoardAction::ShowInProject(path) => {
                self.preferences
                    .workspace
                    .reveal(crate::dock::Panel::Project);
                self.select_asset(&path);
            }
        }
        self.scene_board.invalidate();
    }

    /// One change to the project's scene list, read from disk and written
    /// back, as nominating a main scene is.
    fn edit_scene_list(
        &mut self,
        scene: &Path,
        done: &str,
        edit: impl FnOnce(&mut Project, &Path) -> Result<(), crate::project::ProjectError>,
    ) {
        let Some(root) = self.open_project_root.clone() else {
            return;
        };
        let result = Project::open(&root).and_then(|mut project| {
            edit(&mut project, scene)?;
            Ok(project)
        });
        match result {
            Ok(project) => {
                self.adopt(&project);
                self.console.info(format!(
                    "{} {done} {}",
                    project.name(),
                    scene.strip_prefix(&root).unwrap_or(scene).display()
                ));
            }
            Err(error) => self.report(error.to_string()),
        }
    }
}
