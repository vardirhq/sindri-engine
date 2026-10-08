//! Resolving the scene through the project's Weave styles for a viewport.

use eframe::egui::Rect;
use weave::Viewport as WeaveViewport;

use super::EditorApp;

impl EditorApp {
    /// Styles the edited world in place through the project's Weave
    /// presentation for one view's draw, and returns what takes it off.
    ///
    /// A running game is styled by its session instead, which settled the
    /// world when Play started and lays the pointer's states over the Game
    /// view as it draws. Failures go to the console and the render error
    /// whichever view asked.
    pub(super) fn present_for_editing(
        &mut self,
        editing: bool,
        rect: Rect,
    ) -> Option<sindri_weave::Undo> {
        if self.styles.is_empty() || !self.authoring_enabled() {
            return None;
        }
        let viewport = self.presentation_viewport(editing, rect);
        match self.styles.present(&mut self.world, viewport) {
            Ok(undo) => undo,
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
