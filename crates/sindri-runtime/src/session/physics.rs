//! Both scene solvers advance on the host fixed clock before scripts.

use super::Session;
use crate::RuntimeError;
use sindri_core::World;
use sindri_scene::{ScenePhysics2d, ScenePhysics3d};

impl Session {
    /// Prepares decoded model triangles and binds them for the next fixed step.
    /// Call after asynchronous loading succeeds; rendering remains independent.
    /// Failed preparation retains the previous binding and live solver geometry.
    ///
    /// # Errors
    /// Names invalid CPU preparation or static physics geometry.
    pub fn bind_model_collision(
        &mut self,
        id: sindri_core::AssetId,
        asset: &sindri_assets::ModelAsset,
    ) -> Result<(), crate::ModelCollisionBindingError> {
        let mesh = asset.collision_mesh(sindri_assets::ModelCollisionLimits::default())?;
        let resource = sindri_scene::ModelCollisionGeometry3d::new(mesh.positions, mesh.triangles)?;
        self.physics3d
            .model_collision_bindings_mut()
            .bind(id, std::sync::Arc::new(resource));
        Ok(())
    }

    /// Prepares one volumetric hull from the selected decoded model hierarchy.
    /// Call after asynchronous loading. Convex approximation fills concavities;
    /// failure leaves the previous hull binding and live solver unchanged.
    ///
    /// # Errors
    /// Invalid model preparation or bounded, nonvolumetric hull geometry.
    pub fn bind_model_convex_collision(
        &mut self,
        id: sindri_core::AssetId,
        asset: &sindri_assets::ModelAsset,
    ) -> Result<(), crate::ModelCollisionBindingError> {
        let mesh = asset.collision_mesh(sindri_assets::ModelCollisionLimits {
            vertices: 4_096,
            ..sindri_assets::ModelCollisionLimits::default()
        })?;
        let resource = sindri_scene::ModelConvexGeometry3d::new(mesh.positions)?;
        self.physics3d
            .bind_model_convex_collision(id, std::sync::Arc::new(resource));
        Ok(())
    }

    /// Removes a convex binding; active references fail before the next solve.
    pub fn unbind_model_convex_collision(&mut self, id: &sindri_core::AssetId) {
        self.physics3d.unbind_model_convex_collision(id);
    }

    /// Removes a host binding; active scene references fail before the next solve.
    pub fn unbind_model_collision(&mut self, id: &sindri_core::AssetId) {
        self.physics3d.model_collision_bindings_mut().unbind(id);
    }

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

    /// Queue world-space 3D character movement for the next shared fixed step.
    /// Gameplay supplies displacement; the scene controller owns collision resolution.
    pub fn character_requests3d(&mut self) -> &mut sindri_scene::CharacterRequests3d {
        self.physics3d.character_requests()
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
