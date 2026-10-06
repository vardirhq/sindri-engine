//! Each spawned mechanism keeps its own endpoints, including its original root ID.
use std::time::Duration;

use serde_json::json;
use sindri_core::{EntityData, PrefabDocument, SceneComponent, SceneEntityId, World};
use sindri_physics::{Collider2d, RigidBody2d, RigidBodyKind};

use super::reference::resolve;
use super::{
    DistanceJoint2dComponent, HingeJoint2dComponent, SliderJoint2dComponent, SpringJoint2dComponent,
};
use crate::{SceneExtractor, ScenePhysics2d};

#[test]
fn every_joint_kind_connects_only_inside_its_runtime_spawn() {
    let registry = SceneExtractor::new().unwrap().components().clone();
    for kind in [
        DistanceJoint2dComponent::TYPE_NAME,
        HingeJoint2dComponent::TYPE_NAME,
        SliderJoint2dComponent::TYPE_NAME,
        SpringJoint2dComponent::TYPE_NAME,
    ] {
        let collider = json!({"pieces": [Collider2d::circle(0.1)]});
        let body = RigidBody2d {
            kind: RigidBodyKind::Dynamic,
            ..RigidBody2d::default()
        };
        let prefab: PrefabDocument =
            serde_json::from_value(json!({"format_version": 1, "entities": [
                {"id": "hook", "transform_3d": {"position": [0, 0, 0]},
                    "components": {"sindri.physics2d.collider": collider}},
                {"id": "body", "parent": "hook", "transform_3d": {"position": [1, 0, 0]},
                    "components": {"sindri.physics2d.collider": collider,
                    "sindri.physics2d.rigid_body": body}},
                {"id": "joint", "parent": "hook", "components": {
                    (kind): {"first": "hook", "second": "body", "max_distance": 2.0}}}
            ]}))
            .unwrap();
        let mut world = World::default();
        let first = world.spawn_prefab(&prefab).unwrap();
        let second = world.spawn_prefab(&prefab).unwrap();
        world
            .get_mut(second.root)
            .unwrap()
            .transform_3d
            .as_mut()
            .unwrap()
            .position[0] = 20.0;
        let id = |s| SceneEntityId::new(s).unwrap();
        let owner = first.by_source_id[&id("joint")];
        let body = first.by_source_id[&id("body")];
        let external = world.spawn(EntityData {
            source_id: Some(id("body")),
            ..EntityData::default()
        });
        let mut physics = ScenePhysics2d::top_down().unwrap();
        let step = Duration::from_secs_f32(1.0 / 60.0);
        for _ in 0..30 {
            physics.step(&mut world, &registry, step).unwrap();
        }
        assert_eq!(physics.world().joint_count(), 2, "{kind}");
        assert_eq!(resolve(&world, owner, "hook"), Some(first.root));
        assert_eq!(resolve(&world, owner, "body"), Some(body));
        world.get_mut(body).unwrap().disabled = true;
        assert_eq!(
            resolve(&world, owner, "body"),
            None,
            "must not bind {external:?}"
        );
        physics.step(&mut world, &registry, step).unwrap();
        assert_eq!(physics.world().joint_count(), 1);
        world.get_mut(body).unwrap().disabled = false;
        physics.step(&mut world, &registry, step).unwrap();
        assert_eq!(physics.world().joint_count(), 2);
        world.despawn_recursive(first.root).unwrap();
        physics.step(&mut world, &registry, step).unwrap();
        assert_eq!(physics.world().joint_count(), 1);
    }
}
