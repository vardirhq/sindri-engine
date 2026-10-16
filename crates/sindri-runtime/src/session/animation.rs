//! Animation and sequence playback between scripts and camera following.
use sindri_core::World;
use sindri_decay::AudioCommand;

use super::Session;
use crate::RuntimeError;

impl Session {
    pub(super) fn step_animations(
        &mut self,
        world: &mut World,
        delta_seconds: f32,
        problems: &mut Vec<String>,
    ) -> Result<(), RuntimeError> {
        self.animations
            .advance(world, &self.components, delta_seconds)?;
        // After the scripts, so a sequence a script named this step starts
        // now, and before the cameras, so one that moves a camera is followed.
        let played = self
            .sequences
            .advance(world, &self.components, delta_seconds)?;
        for (_, problem) in &played.problems {
            problems.push(format!("Sequence: {problem}"));
        }
        self.pending_audio
            .extend(played.sounds.into_iter().map(|sound| AudioCommand::Play {
                bus: sound.bus().to_owned(),
                clip: sound.clip,
                volume: sound.volume,
            }));
        Ok(())
    }
}
