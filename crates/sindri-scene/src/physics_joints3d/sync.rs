//! One lifecycle for authored 3D constraints, planned before bodies change.
use super::DistanceJoint3dComponent;
use crate::PhysicsSyncError;
use sindri_core::{ComponentSchemaRegistry, Decoded, EntityId, World};
use sindri_physics::PhysicsWorld3d;
use std::collections::BTreeSet;

#[derive(Clone, Default)]
pub(crate) struct SceneJoints3d {
    owners: BTreeSet<EntityId>,
    /// Decoded once per change rather than once a step.
    distance: Decoded<DistanceJoint3dComponent>,
}

/// The authored constraints of one step, every one already validated.
pub(crate) struct JointPlan3d(Vec<(EntityId, DistanceJoint3dComponent)>);

impl SceneJoints3d {
    /// Reads and validates every authored constraint without touching the
    /// solver, so an invalid joint fails the step before any body changes.
    pub(crate) fn plan(
        &mut self,
        world: &World,
        components: &ComponentSchemaRegistry,
    ) -> Result<JointPlan3d, PhysicsSyncError> {
        let authored: Vec<_> = self
            .distance
            .query(components, world)?
            .into_iter()
            .map(|(owner, joint)| (owner, joint.clone()))
            .collect();
        for (_, joint) in &authored {
            joint.validate()?;
        }
        Ok(JointPlan3d(authored))
    }

    /// Makes, retunes or releases each owner's constraint once bodies exist.
    pub(crate) fn commit(
        &mut self,
        world: &World,
        physics: &mut PhysicsWorld3d,
        JointPlan3d(authored): JointPlan3d,
    ) -> Result<(), PhysicsSyncError> {
        let live: BTreeSet<_> = authored.iter().map(|(owner, _)| *owner).collect();
        for owner in self.owners.difference(&live) {
            physics.remove_owned_joint(*owner);
        }
        self.owners = live;
        for (owner, joint) in authored {
            let endpoints = resolve(world, owner, &joint.first)
                .zip(resolve(world, owner, &joint.second))
                .filter(|(first, second)| {
                    first != second && physics.contains(*first) && physics.contains(*second)
                });
            match endpoints {
                Some((first, second)) if joint.enabled && world.is_active(owner) => {
                    physics.set_distance_joint(owner, joint.joint(first, second))?;
                }
                _ => {
                    physics.remove_owned_joint(owner);
                }
            }
        }
        Ok(())
    }
}

fn resolve(world: &World, owner: EntityId, target: &str) -> Option<EntityId> {
    world
        .resolve_entity_reference(owner, target)
        .filter(|&entity| world.is_active(entity))
}
