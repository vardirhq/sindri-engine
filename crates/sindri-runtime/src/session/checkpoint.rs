//! A copy of a run, to go back to.
//!
//! Everything a session carries from one step to the next — the scripts and
//! their state, both solvers with their warm starts, the screen UI, effects,
//! animations, sequences, the random stream, the save as it stands, which
//! scene is playing — copied, so that a session restored from it and given
//! the same input steps exactly as the original did. What a session is told
//! by its host is not part of it: where saves are written is the host's, so a
//! restored session keeps writing where it was, and whether it is timing its
//! steps is the host's choice too.

use super::Session;

/// A session's state at one step. Taken with [`Session::checkpoint`] beside a
/// copy of the world, and put back with [`Session::restore`].
pub struct Checkpoint(Box<Session>);

impl Session {
    /// A copy of this session's state, for going back to.
    #[must_use]
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint(Box::new(self.copied()))
    }

    /// Puts this session back as it was when `checkpoint` was taken, keeping
    /// where its saves go and whether it is measuring. The world must be put
    /// back to the copy taken with it.
    pub fn restore(&mut self, checkpoint: &Checkpoint) {
        let mut restored = checkpoint.0.copied();
        std::mem::swap(&mut restored.save_backend, &mut self.save_backend);
        restored.measuring = self.measuring;
        restored.scripts.set_measuring(self.measuring);
        *self = restored;
    }

    fn copied(&self) -> Self {
        Self {
            scripts: self.scripts.clone(),
            sources: self.sources.clone(),
            prefabs: self.prefabs.clone(),
            tile_sets: self.tile_sets.clone(),
            profiles: self.profiles.clone(),
            components: self.components.clone(),
            measuring: self.measuring,
            animations: self.animations.clone(),
            sequences: self.sequences.clone(),
            physics: self.physics.clone(),
            physics3d: self.physics3d.clone(),
            screen_ui: self.screen_ui.clone(),
            editing_text: self.editing_text,
            styles: self.styles.clone(),
            text_sizes: self.text_sizes.clone(),
            random: self.random.clone(),
            saves: self.saves.clone(),
            effects: self.effects.clone(),
            gestures: self.gestures.clone(),
            surfaces: self.surfaces.clone(),
            since_written: self.since_written,
            // A checkpoint writes no saves; `restore` hands back the host's.
            save_backend: Box::new(sindri_platform::MemorySaves::new()),
            pending_audio: self.pending_audio.clone(),
            mixer: self.mixer.clone(),
            autoplay_started: self.autoplay_started,
            scenes: self.scenes.clone(),
            loaded: self.loaded.clone(),
            channel: self.channel.clone(),
        }
    }
}
