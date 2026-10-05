//! Material delivery and fixed-step physics use the same editor asset pipeline.

use sindri_core::ComponentSchemaRegistry;

use super::EditorApp;

impl EditorApp {
    pub(super) fn step_physics(
        &mut self,
        components: &ComponentSchemaRegistry,
        delta: std::time::Duration,
    ) -> bool {
        if self.scripts.loading() {
            return false;
        }
        let result = self
            .scripts
            .physics_materials()
            .map_err(sindri_scene::PhysicsSyncError::from)
            .and_then(|materials| {
                self.physics.set_materials(materials);
                self.physics.step(&mut self.world, components, delta)
            });
        if let Err(error) = result {
            self.console.error(format!("Physics: {error}"));
            return false;
        }
        true
    }
}
