//! The physics a script may read and drive.
//!
//! `sindri-physics` owns the simulation; this crate owns what a script is
//! allowed to say about it. The pair is deliberately the *world* and the
//! *events*, not the scene-side driver that keeps them in step: a host with its
//! own driver can still give Decay physics, and `sindri-decay` stays out of the
//! business of deciding when a step happens.
//!
//! Rapier appears nowhere here, which is `docs/physics.md`'s whole point:
//! Sindri exposes Sindri physics.

use sindri_physics::{PhysicsEvent2d, PhysicsWorld2d};

/// The independent 3D simulation and completed events offered to one script pass.
pub struct Physics3d<'a> {
    pub world: &'a mut sindri_physics::PhysicsWorld3d,
    /// Every script observes this borrowed snapshot without draining it.
    pub events: &'a [sindri_physics::PhysicsEvent3d],
}

/// The 2D physics one pass of scripts may reach.
pub struct Physics2d<'a> {
    /// The simulation, for the operations gameplay drives: velocity, impulses.
    pub world: &'a mut PhysicsWorld2d,
    /// What the last step reported, in the order the backend produced it.
    ///
    /// A borrowed slice rather than a queue a script drains, because every
    /// script in the pass sees the same frame's events and none of them may
    /// take an event away from another.
    pub events: &'a [PhysicsEvent2d],
}

/// Scene-owned character requests and results for one pass of scripts.
///
/// Supply alongside `Physics2d` when the host uses scene controllers. Requests
/// apply at the next fixed step; all scripts read the previous step's results.
pub struct Characters2d<'a> {
    pub requests: &'a mut sindri_scene::CharacterRequests2d,
    pub motions: sindri_scene::CharacterMotions2d<'a>,
}
