//! The graphics quality, between the scripts that choose it and the host that
//! draws at it.

use sindri_scene::{GraphicsQuality, QUALITY_BOARD, QUALITY_REQUEST_BOARD, SceneExtractor};

use super::Session;

impl Session {
    /// Applies a script's `Graphics.set_quality` to `scene`, and publishes the
    /// level `scene` draws at for `Graphics.quality` to read.
    ///
    /// Every host that draws calls this after a step, so a settings menu works
    /// the same in each, and a host that lowers the quality itself (a device
    /// that cannot keep up) is heard by the scripts too.
    pub fn sync_graphics_quality(&mut self, scene: &mut SceneExtractor) {
        let board = self.scripts.blackboard_mut();
        let asked = board.get(QUALITY_REQUEST_BOARD, 0.0);
        if asked >= 1.0 {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            if let Some(level) = GraphicsQuality::ALL.get(asked as usize - 1) {
                scene.set_graphics_quality(*level);
            }
            board.set(QUALITY_REQUEST_BOARD, 0.0);
        }
        #[allow(clippy::cast_precision_loss)]
        board.set(QUALITY_BOARD, scene.graphics_quality().index() as f64);
    }
}
