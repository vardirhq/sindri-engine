//! Material delivery and fixed-step physics use the same editor asset pipeline.

use sindri_core::ComponentSchemaRegistry;

use super::EditorApp;

impl EditorApp {
    /// Derived solver state belongs to one run and one loaded scene.
    pub(in crate::native) fn reset_physics(&mut self) {
        self.physics = sindri_scene::ScenePhysics2d::top_down().expect("zero gravity is finite");
        self.physics3d =
            sindri_scene::ScenePhysics3d::new([0.0; 3]).expect("zero gravity is finite");
    }

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
                self.physics.step(&mut self.world, components, delta)?;
                // With the block sets, because an authored voxel collider's
                // blocks are theirs to say the shape of.
                self.physics3d.step_with_tile_sets(
                    &mut self.world,
                    components,
                    Some(self.textures.tile_sets()),
                    delta,
                )
            });
        if let Err(error) = result {
            self.console.error(format!("Physics: {error}"));
            return false;
        }
        true
    }
}
