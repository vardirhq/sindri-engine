//! Copying Rapier's world.
//!
//! A run is recorded by copying the whole simulation every so often, and
//! scrubbing restores a copy and replays from it, which only works if the
//! copy goes on exactly as the original would have. Every part of Rapier's
//! world that carries state from one step to the next — bodies, colliders,
//! joints, islands, both broad and narrow phases with their warm-start
//! impulses, the CCD solver — is copied. The two pipelines are not: they hold
//! the scratch space a step works in and its counters, which the next step
//! rebuilds, so a copy starts with fresh ones.

macro_rules! copy_backend {
    ($name:ident, $rapier:ident) => {
        pub(crate) fn $name(
            world: &$rapier::prelude::PhysicsWorld,
        ) -> $rapier::prelude::PhysicsWorld {
            $rapier::prelude::PhysicsWorld {
                gravity: world.gravity,
                integration_parameters: world.integration_parameters,
                physics_pipeline: $rapier::prelude::PhysicsPipeline::new(),
                collision_pipeline: $rapier::prelude::CollisionPipeline::new(),
                islands: world.islands.clone(),
                broad_phase: world.broad_phase.clone(),
                narrow_phase: world.narrow_phase.clone(),
                bodies: world.bodies.clone(),
                colliders: world.colliders.clone(),
                impulse_joints: world.impulse_joints.clone(),
                multibody_joints: world.multibody_joints.clone(),
                soft_bodies: world.soft_bodies.clone(),
                ccd_solver: world.ccd_solver.clone(),
            }
        }
    };
}

copy_backend!(copy2d, rapier2d);
copy_backend!(copy3d, rapier3d);
