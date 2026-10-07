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
        self.physics3d.step(
            world,
            &self.components,
            std::time::Duration::from_secs_f32(delta_seconds),
        )?;
        Ok(())
    }
}
