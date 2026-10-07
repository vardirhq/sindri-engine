//! Placed mechanisms retain root aliases across scene loading and switching.
use std::{collections::BTreeMap, time::Duration};

use serde_json::json;
use sindri_core::{
    LoadedScenes, PrefabDocument, SceneComponent, SceneDocument, SceneEntityId, World,
};
use sindri_physics::{Collider2d, RigidBody2d, RigidBodyKind};

use super::reference::resolve;
use super::{
    DistanceJoint2dComponent, HingeJoint2dComponent, SliderJoint2dComponent, SpringJoint2dComponent,
};
use crate::{SceneExtractor, ScenePhysics2d};

fn library(kind: &str) -> BTreeMap<String, PrefabDocument> {
    let collider = json!({"pieces": [Collider2d::circle(0.1)]});
    let body = RigidBody2d {
        kind: RigidBodyKind::Dynamic,
        ..RigidBody2d::default()
    };
    let inner = serde_json::from_value(json!({"format_version": 1, "entities": [
        {"id": "hook", "components": {"sindri.physics2d.collider": collider}},
        {"id": "body", "parent": "hook", "components": {
            "sindri.physics2d.collider": collider, "sindri.physics2d.rigid_body": body}},
        {"id": "joint", "parent": "hook", "components": {
            (kind): {"first": "hook", "second": "body", "max_distance": 2.0}}}
    ]}))
    .unwrap();
    let outer = serde_json::from_value(json!({"format_version": 1, "entities": [
        {"id": "assembly"}, {"id": "mechanism", "parent": "assembly",
            "prefab": {"source": "inner.prefab"}}
    ]}))
    .unwrap();
    BTreeMap::from([
        ("inner.prefab".into(), inner),
        ("outer.prefab".into(), outer),
    ])
}

#[test]
fn placed_nested_roots_resolve_for_every_joint_kind_and_scene_namespace() {
    let registry = SceneExtractor::new().unwrap().components().clone();
    for kind in [
        DistanceJoint2dComponent::TYPE_NAME,
        HingeJoint2dComponent::TYPE_NAME,
        SliderJoint2dComponent::TYPE_NAME,
        SpringJoint2dComponent::TYPE_NAME,
    ] {
        let prefabs = library(kind);
        let scene: SceneDocument =
            serde_json::from_value(json!({"format_version": sindri_core::SCENE_FORMAT_VERSION,
            "entities": [{"id": "first", "prefab": {"source": "outer.prefab"}},
                {"id": "second", "prefab": {"source": "outer.prefab"}}]}))
            .unwrap();
        let mut world = World::default();
        let mut loaded = LoadedScenes::new();
        loaded
            .enter_keeping_identities_with(&mut world, "entry", &scene, &prefabs)
            .unwrap();
        loaded
            .load_with(&mut world, "other", &scene, &prefabs)
            .unwrap();
        let id = |s| SceneEntityId::new(s).unwrap();
        let owner = world
            .entity_for_source_id(&id("first/mechanism/joint"))
            .unwrap();
        let hook = world.entity_for_source_id(&id("first/mechanism")).unwrap();
        let body = world
            .entity_for_source_id(&id("first/mechanism/body"))
            .unwrap();
        assert_eq!(resolve(&world, owner, "hook"), Some(hook));
        let mut physics = ScenePhysics2d::top_down().unwrap();
        let step = Duration::from_millis(16);
        physics.step(&mut world, &registry, step).unwrap();
        assert_eq!(physics.world().joint_count(), 2, "{kind}");
        world.get_mut(hook).unwrap().disabled = true;
        assert_eq!(resolve(&world, owner, "hook"), None);
        physics.step(&mut world, &registry, step).unwrap();
        assert_eq!(physics.world().joint_count(), 1);
        world.get_mut(hook).unwrap().disabled = false;
        physics.step(&mut world, &registry, step).unwrap();
        assert_eq!(physics.world().joint_count(), 2);
        loaded.go_to(&mut world, "other").unwrap();
        let other = world
            .entity_for_source_id(&id("other/first/mechanism/joint"))
            .unwrap();
        assert_eq!(resolve(&world, owner, "hook"), None);
        assert_eq!(
            resolve(&world, other, "hook"),
            world.entity_for_source_id(&id("other/first/mechanism"))
        );
        physics.step(&mut world, &registry, step).unwrap();
        assert_eq!(physics.world().joint_count(), 2);
        loaded.go_to(&mut world, "entry").unwrap();
        world.despawn_recursive(body).unwrap();
        physics.step(&mut world, &registry, step).unwrap();
        assert_eq!(physics.world().joint_count(), 1);
        let authored = World::from_scene_with(&scene, &prefabs).unwrap().world;
        let saved = authored.to_scene_with(&prefabs).unwrap();
        assert!(!serde_json::to_string(&saved).unwrap().contains("aliases"));
        let restored = World::from_scene_with(&saved, &prefabs).unwrap().world;
        let restored_owner = restored
            .entity_for_source_id(&id("first/mechanism/joint"))
            .unwrap();
        assert_eq!(
            resolve(&restored, restored_owner, "hook"),
            restored.entity_for_source_id(&id("first/mechanism"))
        );
    }
}

#[test]
fn an_inactive_canonical_path_shadows_a_placed_root_alias() {
    let prefabs = library(DistanceJoint2dComponent::TYPE_NAME);
    let scene: SceneDocument =
        serde_json::from_value(json!({"format_version": sindri_core::SCENE_FORMAT_VERSION,
        "entities": [{"id": "first", "prefab": {"source": "outer.prefab"}},
            {"id": "first/mechanism/hook", "disabled": true}]}))
        .unwrap();
    let world = World::from_scene_with(&scene, &prefabs).unwrap().world;
    let owner = world
        .entity_for_source_id(&SceneEntityId::new("first/mechanism/joint").unwrap())
        .unwrap();
    assert_eq!(resolve(&world, owner, "hook"), None);
}
