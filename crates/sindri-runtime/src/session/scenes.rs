//! Moving between the scenes a project carries.

use sindri_core::World;

use super::Session;
use crate::RuntimeError;

impl Session {
    /// Performs a scene change a script asked for, if one did.
    ///
    /// Between frames, never inside one: the script that asked is running in
    /// the scene being left, from a world this rearranges underneath it. The
    /// scene it came from is switched off rather than unloaded, so walking back
    /// in finds it as it was.
    pub(super) fn follow_scene_request(&mut self, world: &mut World) -> Result<(), RuntimeError> {
        let Some(channel) = self.channel.as_mut() else {
            return Ok(());
        };
        let Some(wanted) = channel.take() else {
            return Ok(());
        };
        let fresh = channel.take_fresh();
        if fresh {
            // Started over: what the scene was is thrown away first, so it
            // loads from its document as it did the first time, even when it
            // is the scene already being played.
            self.loaded.unload(world, &wanted).map_err(|source| {
                sindri_core::SceneSwitchError::Unload {
                    scene: wanted.clone(),
                    source,
                }
            })?;
        } else if self.loaded.active() == Some(wanted.as_str()) {
            // Already there. Not an error: two doors into one room, or a script
            // asking twice, should be a no-op rather than a reload that threw
            // the room's state away.
            return Ok(());
        }
        let document = self
            .scenes
            .get(&wanted)
            .ok_or_else(|| RuntimeError::UnknownScene(wanted.clone()))?;
        self.loaded
            .enter_with(world, &wanted, document, &self.prefabs)?;
        channel.now_playing(wanted);
        Ok(())
    }
}
