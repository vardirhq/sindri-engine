//! Both scene solvers advance on the host fixed clock before scripts.

use super::Session;
use crate::RuntimeError;
use sindri_core::World;
use sindri_scene::{ScenePhysics2d, ScenePhysics3d};

impl Session {
    /// The 2D solver, for whatever draws what it holds.
    #[must_use]
    pub const fn physics(&self) -> &ScenePhysics2d {
        &self.physics
    }

    /// The 3D solver, for whatever draws what it holds.
    #[must_use]
    pub const fn physics3d(&self) -> &ScenePhysics3d {
        &self.physics3d
    }

    pub(super) fn step_physics(
        &mut self,
        world: &mut World,
        delta_seconds: f32,
    ) -> Result<(), RuntimeError> {
        self.sync_physics_materials()?;
        self.physics.step(
            world,
            &self.components,
            std::time::Duration::from_secs_f32(delta_seconds),
        )?;
        // The block sets decide what an authored voxel collider's blocks are.
        let tile_sets = (!self.tile_sets.is_empty()).then_some(&self.tile_sets);
        self.physics3d.step_with_tile_sets(
            world,
            &self.components,
            tile_sets,
            std::time::Duration::from_secs_f32(delta_seconds),
        )?;
        Ok(())
    }
}
