//! Authored spawn-window validation and real typed prefab materialization.

#[path = "physics3d_controls/support.rs"]
mod support;

use decay_runtime::Value;
use serde_json::json;
use sindri_core::{EntityData, PrefabDocument, SceneComponent, SceneEntity, SceneEntityId};
use sindri_decay::{
    Physics3d, PrefabSources, ScriptComponent, ScriptFrame, ScriptSources, Scripts,
};
use sindri_platform::InputState;
use sindri_scene::{Collider3dComponent, RigidBody2dComponent, RigidBody3dComponent};
use support::{Fixture, near, reference};

#[test]
fn pending_reads_are_copied_setters_and_impulses_resolve_in_order() {
    let mut fixture = Fixture::new();
    let actor = fixture.actor("dynamic", false);
    let authored = fixture.world.get(actor).unwrap().components.clone();
    fixture
        .call(
            actor,
            "set_velocity",
            &[reference(actor), Value::Vec3([2.0, 0.0, 0.0])],
            true,
        )
        .unwrap();
    fixture
        .call(
            actor,
            "apply_impulse",
            &[reference(actor), Value::Vec3([1.0, 0.0, 0.0])],
            true,
        )
        .unwrap();
    assert_eq!(
        fixture
            .call(actor, "velocity", &[reference(actor)], true)
            .unwrap(),
        Value::Vec3([2.0, 0.0, 0.0])
    );
    assert!(!fixture.physics.world().contains(actor));
    fixture.step();
    let velocity = fixture.physics.world().linear_velocity(actor).unwrap();
    assert!(velocity[0] > 2.0);
    near(
        fixture.world.world_transform(actor).unwrap().position,
        velocity.map(|v| v * 0.01),
    );
    assert_eq!(fixture.world.get(actor).unwrap().components, authored);
}

#[test]
fn authored_starts_locks_and_wrong_kinds_are_checked_before_materialization() {
    let mut fixture = Fixture::new();
    let actor = fixture.actor("kinematic_velocity", false);
    let body = fixture
        .world
        .get_mut(actor)
        .unwrap()
        .components
        .get_mut(RigidBody3dComponent::TYPE_NAME)
        .unwrap();
    body["linear_velocity"] = json!([3.0, 2.0, 1.0]);
    body["angular_velocity"] = json!([1.0, 2.0, 3.0]);
    body["lock_rotation"] = json!(true);
    assert_eq!(
        fixture
            .call(actor, "velocity", &[reference(actor)], true)
            .unwrap(),
        Value::Vec3([3.0, 2.0, 1.0])
    );
    fixture
        .call(
            actor,
            "set_angular_velocity",
            &[reference(actor), Value::Vec3([9.0; 3])],
            true,
        )
        .unwrap();
    assert_eq!(
        fixture
            .call(actor, "angular_velocity", &[reference(actor)], true)
            .unwrap(),
        Value::Vec3([0.0; 3])
    );
    assert!(
        fixture
            .call(
                actor,
                "apply_impulse",
                &[reference(actor), Value::Vec3([1.0; 3])],
                true
            )
            .is_err()
    );
    fixture.step();
    near(
        fixture.physics.world().angular_velocity(actor).unwrap(),
        [0.0; 3],
    );
    let static_body = fixture.actor("static", false);
    assert!(
        fixture
            .call(
                static_body,
                "set_velocity",
                &[reference(static_body), Value::Vec3([1.0; 3])],
                true
            )
            .is_err()
    );
}

#[test]
fn invalid_authoring_rejects_requests_without_poisoning_a_corrected_spawn() {
    let mut fixture = Fixture::new();
    let actor = fixture.actor("dynamic", false);
    let authored = fixture.world.get(actor).unwrap().components.clone();
    for (name, payload) in [
        (RigidBody3dComponent::TYPE_NAME, json!({"kind": "unknown"})),
        (Collider3dComponent::TYPE_NAME, json!({"pieces": []})),
        (RigidBody2dComponent::TYPE_NAME, json!({})),
    ] {
        fixture
            .world
            .get_mut(actor)
            .unwrap()
            .components
            .insert(name.into(), payload);
        for call in ["velocity", "set_velocity"] {
            let args = if call == "velocity" {
                vec![reference(actor)]
            } else {
                vec![reference(actor), Value::Vec3([99.0; 3])]
            };
            assert!(fixture.call(actor, call, &args, true).is_err());
        }
        fixture
            .world
            .get_mut(actor)
            .unwrap()
            .components
            .clone_from(&authored);
    }
    fixture
        .world
        .get_mut(actor)
        .unwrap()
        .components
        .remove(Collider3dComponent::TYPE_NAME);
    assert!(
        fixture
            .call(actor, "velocity", &[reference(actor)], true)
            .is_err()
    );
    fixture
        .world
        .get_mut(actor)
        .unwrap()
        .components
        .clone_from(&authored);
    fixture.step();
    near(
        fixture.physics.world().linear_velocity(actor).unwrap(),
        [0.0; 3],
    );
}

#[test]
fn typed_prefab_spawn_queues_before_the_first_physics_step() {
    let mut fixture = Fixture::new();
    let template = fixture.actor("dynamic", false);
    let components = fixture.world.get(template).unwrap().components.clone();
    fixture.world.despawn_recursive(template).unwrap();
    let mut root = SceneEntity::new(SceneEntityId::new("body").unwrap());
    root.name = Some("Spawned body".into());
    root.components = components;
    root.transform_3d = Some(sindri_core::Transform3D::default());
    let mut prefabs = PrefabSources::new();
    prefabs.insert("body.prefab", PrefabDocument::single(root));
    let spawner = fixture.world.spawn(EntityData {
        components: [(
            ScriptComponent::TYPE_NAME.into(),
            json!({"source": "spawn.decay", "script": "Spawn3d", "properties": {"body_prefab": "body.prefab"}}),
        )]
        .into(),
        ..EntityData::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert("spawn.decay", include_str!("physics3d_spawn.decay"));
    let mut scripts = Scripts::new();
    let input = InputState::default();
    let (world, events) = fixture.physics.for_scripts();
    let report = scripts.advance(
        &mut fixture.world,
        fixture.extractor.components(),
        ScriptFrame::new(&sources, &input, 0.01)
            .with_prefabs(&prefabs)
            .with_physics3d(Physics3d { world, events }),
    );
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    assert_eq!(scripts.field(spawner, "copied"), Some(&Value::Bool(true)));
    let actor = fixture
        .world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some("Spawned body"))
        .unwrap()
        .0;
    assert!(!fixture.physics.world().contains(actor));
    fixture.step();
    near(
        fixture.physics.world().linear_velocity(actor).unwrap(),
        [4.0, 0.0, 0.0],
    );
    near(
        fixture.world.world_transform(actor).unwrap().position,
        [0.04, 0.0, 0.0],
    );
}
