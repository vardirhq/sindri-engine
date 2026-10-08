//! One shared lifecycle for every authored constraint kind.
use super::reference::resolve;
use super::{
    DistanceJoint2dComponent, HingeJoint2dComponent, SliderJoint2dComponent, SpringJoint2dComponent,
};
use crate::PhysicsSyncError;
use sindri_core::{ComponentSchemaRegistry, EntityId, World};
use sindri_physics::{
    DistanceJoint2d, HingeJoint2d, PhysicsError, PhysicsWorld2d, SliderJoint2d, SpringJoint2d,
};
use std::collections::BTreeSet;

#[derive(Clone, Default)]
pub(crate) struct SceneJoints2d {
    owners: BTreeSet<EntityId>,
}

enum AuthoredJoint {
    Distance(DistanceJoint2dComponent),
    Hinge(HingeJoint2dComponent),
    Slider(SliderJoint2dComponent),
    Spring(SpringJoint2dComponent),
}

impl AuthoredJoint {
    fn references(&self) -> (&str, &str) {
        match self {
            Self::Distance(joint) => (&joint.first, &joint.second),
            Self::Hinge(joint) => (&joint.first, &joint.second),
            Self::Slider(joint) => (&joint.first, &joint.second),
            Self::Spring(joint) => (&joint.first, &joint.second),
        }
    }
    fn enabled(&self) -> bool {
        match self {
            Self::Distance(joint) => joint.enabled,
            Self::Hinge(joint) => joint.enabled,
            Self::Slider(joint) => joint.enabled,
            Self::Spring(joint) => joint.enabled,
        }
    }
    fn validate(&self) -> Result<(), PhysicsError> {
        match self {
            Self::Distance(joint) => joint.validate(),
            Self::Hinge(joint) => joint.settings.validate(),
            Self::Slider(joint) => joint.settings.validate(),
            Self::Spring(joint) => joint.settings.validate(),
        }
    }
    fn apply(
        self,
        owner: EntityId,
        first: EntityId,
        second: EntityId,
        physics: &mut PhysicsWorld2d,
    ) -> Result<(), PhysicsError> {
        match self {
            Self::Distance(joint) => physics.set_distance_joint(
                owner,
                DistanceJoint2d::new(first, second, joint.max_distance),
            ),
            Self::Hinge(joint) => physics.set_hinge_joint(
                owner,
                HingeJoint2d {
                    first,
                    second,
                    settings: joint.settings,
                },
            ),
            Self::Slider(joint) => physics.set_slider_joint(
                owner,
                SliderJoint2d {
                    first,
                    second,
                    settings: joint.settings,
                },
            ),
            Self::Spring(joint) => physics.set_spring_joint(
                owner,
                SpringJoint2d {
                    first,
                    second,
                    settings: joint.settings,
                },
            ),
        }
    }
}

impl SceneJoints2d {
    pub(crate) fn synchronize(
        &mut self,
        world: &World,
        components: &ComponentSchemaRegistry,
        physics: &mut PhysicsWorld2d,
    ) -> Result<(), PhysicsSyncError> {
        let mut authored: Vec<_> = components
            .query::<DistanceJoint2dComponent>(world)?
            .into_iter()
            .map(|(owner, joint)| (owner, AuthoredJoint::Distance(joint)))
            .collect();
        authored.extend(
            components
                .query::<HingeJoint2dComponent>(world)?
                .into_iter()
                .map(|(owner, joint)| (owner, AuthoredJoint::Hinge(joint))),
        );
        authored.extend(
            components
                .query::<SliderJoint2dComponent>(world)?
                .into_iter()
                .map(|(owner, joint)| (owner, AuthoredJoint::Slider(joint))),
        );
        authored.extend(
            components
                .query::<SpringJoint2dComponent>(world)?
                .into_iter()
                .map(|(owner, joint)| (owner, AuthoredJoint::Spring(joint))),
        );
        let mut live = BTreeSet::new();
        for (owner, joint) in &authored {
            if !live.insert(*owner) {
                return Err(PhysicsSyncError::ConflictingJointComponents(*owner));
            }
            joint.validate()?;
        }
        for owner in self.owners.difference(&live) {
            physics.remove_owned_joint(*owner);
        }
        self.owners = live;
        for (owner, joint) in authored {
            if !joint.enabled() {
                physics.remove_owned_joint(owner);
                continue;
            }
            let (first, second) = joint.references();
            let endpoints = resolve(world, owner, first).zip(resolve(world, owner, second));
            if let Some((first, second)) = endpoints
                .filter(|(first, second)| physics.contains(*first) && physics.contains(*second))
            {
                joint.apply(owner, first, second, physics)?;
            } else {
                physics.remove_owned_joint(owner);
            }
        }
        Ok(())
    }
}
