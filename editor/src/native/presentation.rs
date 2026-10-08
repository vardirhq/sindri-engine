//! Resolving the scene through the project's Weave styles for a viewport.

use eframe::egui::Rect;
use weave::Viewport as WeaveViewport;

use super::EditorApp;

impl EditorApp {
    /// Resolves the authored world through the project's Weave presentation.
    ///
    /// Kept outside `render_view` because resolving presentation is one concern
    /// of its own and because failures need the same console/render-error path
    /// whichever viewport asked for them.
    pub(super) fn resolve_presentation(
        &mut self,
        editing: bool,
        rect: Rect,
    ) -> Option<sindri_core::World> {
        if self.styles.is_empty() {
            return None;
        }
        // A running game is styled by its session, which settled the world
        // when Play started and lays the pointer's states over the Game view
        // in place as it draws: what either view shows of a run is the world.
        if !self.authoring_enabled() {
            return None;
        }
        let viewport = self.presentation_viewport(editing, rect);
        let presented = self.styles.resolve(&self.world, viewport);
        match presented {
            Ok(world) => Some(world),
            Err(error) => {
                let failure = format!("Weave: {error}");
                self.console.fail(&failure, None);
                if self.render_error.is_none() {
                    self.render_error = Some(failure);
                }
                None
            }
        }
    }

    /// The logical screen Weave styles a run for: the Game view's, as last
    /// drawn, or the screen the editor opens at before it has been.
    pub(super) fn game_presentation_viewport(&self) -> WeaveViewport {
        // The size the views' targets start at, in points: small enough
        // that the conversion is exact.
        let rect = self.last_game_view.unwrap_or_else(|| {
            Rect::from_min_size(eframe::egui::Pos2::ZERO, eframe::egui::vec2(960.0, 540.0))
        });
        self.presentation_viewport(false, rect)
    }

    /// The logical screen dimensions Weave resolves against.
    ///
    /// A named device uses its real logical size, not the number of editor
    /// points its preview happened to fit into. In Free mode the Game view is
    /// the screen. The Scene view follows that Game rectangle when one has been
    /// drawn, so both views choose the same media queries while shown together.
    pub(super) fn presentation_viewport(&self, editing: bool, rect: Rect) -> WeaveViewport {
        let (width, height) = self.game_device.size.unwrap_or_else(|| {
            if editing {
                self.game_view_rect
                    .map_or((rect.width(), rect.height()), |game| {
                        (game.width(), game.height())
                    })
            } else {
                (rect.width(), rect.height())
            }
        });
        WeaveViewport {
            width: width.max(1.0),
            height: height.max(1.0),
        }
    }
}
