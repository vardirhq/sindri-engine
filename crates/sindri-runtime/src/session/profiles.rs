//! Authored reusable assets supplied by native and browser loaders.

use sindri_decay::{PrefabSources, ProfileSources};

use super::Session;

impl Session {
    /// The prefabs this project's scripts can spawn.
    ///
    /// A build had no way to be given any, so `World.spawn` in a shipped game
    /// answered that the prefab was missing while the same scene spawned
    /// correctly in the editor. A project whose enemies are prefabs is every
    /// project that spawns anything.
    #[must_use]
    pub fn with_prefabs(mut self, prefabs: PrefabSources) -> Self {
        self.prefabs = prefabs;
        self
    }

    #[must_use]
    pub fn with_profiles(mut self, profiles: ProfileSources) -> Self {
        self.profiles = profiles;
        self
    }

    pub(super) fn sync_physics_materials(&mut self) -> Result<(), crate::RuntimeError> {
        let profiles = self
            .profiles
            .ids()
            .filter_map(|id| self.profiles.get(id).map(|profile| (id, profile)));
        let materials = sindri_scene::PhysicsMaterialSources::from_profiles(profiles)
            .map_err(sindri_scene::PhysicsSyncError::from)?;
        self.physics.set_materials(materials);
        Ok(())
    }
}
