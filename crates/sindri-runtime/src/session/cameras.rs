//! One camera phase after scripts and animation, with the synchronized solver.

use super::Session;
use sindri_core::World;

impl Session {
    pub(super) fn step_cameras(
        &self,
        world: &mut World,
        delta_seconds: f32,
        problems: &mut Vec<String>,
    ) {
        sindri_scene::update_camera_behaviors(world, delta_seconds);
        for (entity, problem) in
            sindri_scene::update_orbit_cameras(world, self.physics3d.world(), delta_seconds)
        {
            problems.push(format!("Camera orbit {entity:?}: {problem}"));
        }
    }
}
