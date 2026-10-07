//! Both scene solvers advance on the host fixed clock before scripts.

use super::Session;
use crate::error::CausewayError;
use sindri_core::World;

impl Session {
    pub(super) fn step_physics(
        &mut self,
        world: &mut World,
        delta_seconds: f32,
    ) -> Result<(), CausewayError> {
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
