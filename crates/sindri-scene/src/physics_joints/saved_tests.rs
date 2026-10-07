//! Reopened spawned constraints use saved endpoints without runtime identity.
use super::{
    DistanceJoint2dComponent, HingeJoint2dComponent, SliderJoint2dComponent, SpringJoint2dComponent,
};
use crate::{SceneExtractor, ScenePhysics2d};
use serde_json::json;
use sindri_core::{LoadedScenes, NoPrefabs, PrefabDocument, SceneComponent, SceneEntityId, World};
use sindri_physics::{Collider2d, RigidBody2d, RigidBodyKind};
use std::time::Duration;

#[test]
fn every_joint_kind_reopens_with_isolated_saved_endpoints() {
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
                {"id": "hook", "components": {"sindri.physics2d.collider": collider}},
                {"id": "body", "parent": "hook", "components": {
                    "sindri.physics2d.collider": collider, "sindri.physics2d.rigid_body": body}},
                {"id": "joint", "parent": "hook", "components": {
                    (kind): {"first": "hook", "second": "body", "max_distance": 2.0, "extra": 42}}}
            ]}))
            .unwrap();
        let mut world = World::default();
        let first = world.spawn_prefab(&prefab).unwrap();
        world.spawn_prefab(&prefab).unwrap();
        world.assign_missing_source_ids("saved/assigned").unwrap();
        let owner = first.by_source_id[&SceneEntityId::new("joint").unwrap()];
        let target = first.by_source_id[&SceneEntityId::new("body").unwrap()];
        world.get_mut(target).unwrap().source_id = Some(SceneEntityId::new("saved/body").unwrap());
        let mut shadow = world.get(target).unwrap().clone();
        shadow.source_id = Some(SceneEntityId::new("saved/saved/body").unwrap());
        shadow.prefab_identity = None;
        shadow.parent = None;
        shadow.children.clear();
        world.spawn(shadow);
        let target_id = world.get(target).unwrap().source_id.clone().unwrap();
        let owner_id = world.get(owner).unwrap().source_id.clone().unwrap();
        let document = world
            .to_scene_with_references(&NoPrefabs, &registry)
            .unwrap();
        assert_eq!(
            document.entity(&owner_id).unwrap().components[kind]["extra"],
            42
        );
        let text = document.to_canonical_json().unwrap();
        let document = sindri_core::SceneDocument::from_json(&text).unwrap();
        let mut restored = World::default();
        LoadedScenes::new()
            .enter_with(&mut restored, "reopened", &document, &NoPrefabs)
            .unwrap();
        assert!(
            restored
                .entities()
                .all(|(_, data)| data.prefab_identity.is_none())
        );
        let mut physics = ScenePhysics2d::top_down().unwrap();
        let step = Duration::from_millis(16);
        physics.step(&mut restored, &registry, step).unwrap();
        assert_eq!(physics.world().joint_count(), 2, "{kind}");
        let target = restored
            .entity_for_source_id(
                &SceneEntityId::new(format!("reopened/{}", target_id.as_str())).unwrap(),
            )
            .unwrap();
        restored.get_mut(target).unwrap().disabled = true;
        physics.step(&mut restored, &registry, step).unwrap();
        assert_eq!(physics.world().joint_count(), 1);
        restored.get_mut(target).unwrap().disabled = false;
        physics.step(&mut restored, &registry, step).unwrap();
        assert_eq!(physics.world().joint_count(), 2);
        let owner = restored
            .entity_for_source_id(
                &SceneEntityId::new(format!("reopened/{}", owner_id.as_str())).unwrap(),
            )
            .unwrap();
        restored.despawn_recursive(owner).unwrap();
        physics.step(&mut restored, &registry, step).unwrap();
        assert_eq!(physics.world().joint_count(), 1);
    }
}
