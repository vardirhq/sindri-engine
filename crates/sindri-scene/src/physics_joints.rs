//! Authored constraints resolve after every body has synchronized.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sindri_core::{ComponentSchemaRegistry, EntityId, SceneComponent, SceneEntityId, World};
use sindri_physics::{DistanceJoint2d, HingeJoint2d, HingeSettings2d, PhysicsWorld2d};

use crate::PhysicsSyncError;

/// A separate entity owns the constraint, so several joints can connect a body.
/// Endpoints are stable scene IDs, never serialized runtime handles. Empty,
/// missing or inactive endpoints suspend the joint until they become available.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct DistanceJoint2dComponent {
    #[serde(default)]
    pub first: String,
    #[serde(default)]
    pub second: String,
    #[serde(default = "default_distance")]
    pub max_distance: f32,
}

const fn default_distance() -> f32 {
    1.0
}

impl SceneComponent for DistanceJoint2dComponent {
    const TYPE_NAME: &'static str = "sindri.physics2d.distance_joint";
}

/// Body-local anchors, optional relative angle limits and a velocity motor.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct HingeJoint2dComponent {
    #[serde(default)]
    pub first: String,
    #[serde(default)]
    pub second: String,
    #[serde(flatten)]
    pub settings: HingeSettings2d,
}

impl SceneComponent for HingeJoint2dComponent {
    const TYPE_NAME: &'static str = "sindri.physics2d.hinge_joint";
}

#[derive(Default)]
pub(crate) struct SceneJoints2d {
    owners: BTreeSet<EntityId>,
}

impl SceneJoints2d {
    pub(crate) fn synchronize(
        &mut self,
        world: &World,
        components: &ComponentSchemaRegistry,
        physics: &mut PhysicsWorld2d,
    ) -> Result<(), PhysicsSyncError> {
        let authored = components.query::<DistanceJoint2dComponent>(world)?;
        let hinges = components.query::<HingeJoint2dComponent>(world)?;
        let mut live: BTreeSet<_> = authored.iter().map(|(owner, _)| *owner).collect();
        for (owner, joint) in &hinges {
            if !live.insert(*owner) {
                return Err(PhysicsSyncError::ConflictingJointComponents(*owner));
            }
            joint.settings.validate()?;
        }
        for owner in self.owners.difference(&live) {
            physics.remove_owned_joint(*owner);
        }
        self.owners = live;
        for (owner, joint) in authored {
            let endpoints =
                resolve(world, owner, &joint.first).zip(resolve(world, owner, &joint.second));
            if let Some((first, second)) = endpoints
                .filter(|(first, second)| physics.contains(*first) && physics.contains(*second))
            {
                physics.set_distance_joint(
                    owner,
                    DistanceJoint2d::new(first, second, joint.max_distance),
                )?;
            } else {
                physics.remove_owned_joint(owner);
            }
        }
        for (owner, joint) in hinges {
            let endpoints =
                resolve(world, owner, &joint.first).zip(resolve(world, owner, &joint.second));
            if let Some((first, second)) = endpoints
                .filter(|(first, second)| physics.contains(*first) && physics.contains(*second))
            {
                physics.set_hinge_joint(
                    owner,
                    HingeJoint2d {
                        first,
                        second,
                        settings: joint.settings,
                    },
                )?;
            } else {
                physics.remove_owned_joint(owner);
            }
        }
        Ok(())
    }
}

/// Searches the owner's ID namespace before the containing scene. An inactive
/// local target never falls through to a similarly named external entity.
fn resolve(world: &World, owner: EntityId, target: &str) -> Option<EntityId> {
    if target.is_empty() {
        return None;
    }
    let within = boundary(world, owner);
    let usable = |entity| boundary(world, entity) == within && world.is_active(entity);
    // A placed prefab's root takes its instance ID, not its original root ID.
    let mut ancestor = Some(owner);
    while let Some(entity) = ancestor {
        let data = world.get(entity)?;
        if data
            .prefab
            .as_ref()
            .is_some_and(|link| link.root && link.path.as_str() == target)
        {
            return usable(entity).then_some(entity);
        }
        ancestor = data.parent;
    }
    let mut namespace = world
        .get(owner)?
        .source_id
        .as_ref()
        .map_or("", SceneEntityId::as_str);
    while let Some((prefix, _)) = namespace.rsplit_once('/') {
        let key = SceneEntityId::new(format!("{prefix}/{target}")).ok()?;
        if let Some(entity) = world.entity_for_source_id(&key) {
            return usable(entity).then_some(entity);
        }
        namespace = prefix;
    }
    let entity = world.entity_for_source_id(&SceneEntityId::new(target).ok()?)?;
    usable(entity).then_some(entity)
}

/// Loaded scenes have an anonymous root; raw scenes share the unparented scope.
fn boundary(world: &World, mut entity: EntityId) -> Option<EntityId> {
    while let Some(parent) = world.get(entity)?.parent {
        entity = parent;
    }
    world.get(entity)?.source_id.is_none().then_some(entity)
}

#[cfg(test)]
mod tests;
