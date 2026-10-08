//! The session as the platform's [`Game`]: what the shipped hosts drive.

use sindri_platform::{FrameContext, Game};

use super::Session;
use crate::RuntimeError;

impl Game for Session {
    type Error = RuntimeError;

    fn fixed_update(&mut self, context: &mut FrameContext<'_>) -> Result<(), Self::Error> {
        self.start_autoplay(context.world, context.audio)?;
        #[allow(clippy::cast_precision_loss)]
        let viewport = (context.viewport[0] as f32, context.viewport[1] as f32);
        let report = self.step(
            context.world,
            context.input,
            viewport,
            context.time.delta.as_secs_f32(),
        )?;
        // A shipped game has nowhere to put these but its log.
        report.log();
        self.write_saves(context.time.delta.as_secs_f32(), false);
        self.flush_audio(context.audio)
    }

    /// The last chance to keep what a run earned.
    fn stop(&mut self, _context: &mut FrameContext<'_>) -> Result<(), Self::Error> {
        self.write_saves(0.0, true);
        Ok(())
    }
}
