//! Typed 3D controls use the real scene driver and independent host context.

#[path = "physics3d_controls/support.rs"]
mod support;

use decay_runtime::Value;
use serde_json::json;
use sindri_core::SceneComponent;
use sindri_decay::{Physics3d, ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_scene::RigidBody3dComponent;
use support::{Fixture, near, reference};

#[test]
fn vec3_controls_survive_scene_sync_without_rebuilding_authored_motion() {
    let mut fixture = Fixture::new();
    let actor = fixture.actor("dynamic", false);
    fixture.step();
    let authored = fixture.world.get(actor).unwrap().components.clone();
    fixture
        .call(
            actor,
            "set_velocity",
            &[reference(actor), Value::Vec3([1.0, 2.0, 3.0])],
            true,
        )
        .unwrap();
    fixture
        .call(
            actor,
            "set_angular_velocity",
            &[reference(actor), Value::Vec3([0.2, 0.3, 0.4])],
            true,
        )
        .unwrap();
    assert_eq!(
        fixture
            .call(actor, "velocity", &[reference(actor)], true)
            .unwrap(),
        Value::Vec3([1.0, 2.0, 3.0])
    );
    fixture
        .call(
            actor,
            "apply_impulse",
            &[reference(actor), Value::Vec3([1.0, 0.0, 0.0])],
            true,
        )
        .unwrap();
    let velocity = fixture.physics.world().linear_velocity(actor).unwrap();
    assert!(velocity[0] > 1.0);
    fixture.step();
    near(
        fixture.physics.world().linear_velocity(actor).unwrap(),
        velocity,
    );
    near(
        fixture.world.world_transform(actor).unwrap().position,
        velocity.map(|axis| axis * 0.01),
    );
    assert_eq!(fixture.world.get(actor).unwrap().components, authored);
    let Value::Vec3(angular) = fixture
        .call(actor, "angular_velocity", &[reference(actor)], true)
        .unwrap()
    else {
        panic!("expected Vec3")
    };
    assert!((angular[0] - 0.2).abs() < 0.002);
}

#[test]
fn invalid_vectors_arity_context_and_handles_leave_live_motion_untouched() {
    let mut fixture = Fixture::new();
    let actor = fixture.actor("dynamic", false);
    fixture.step();
    for call in ["set_velocity", "set_angular_velocity", "apply_impulse"] {
        for vector in [
            Value::Vec3([f64::NAN, 0.0, 0.0]),
            Value::Vec3([0.0, f64::INFINITY, 0.0]),
            Value::Vec3([0.0, 0.0, f64::MAX]),
            Value::Vec2([1.0, 2.0]),
        ] {
            assert!(
                fixture
                    .call(actor, call, &[reference(actor), vector], true)
                    .is_err()
            );
        }
        assert!(
            fixture
                .call(actor, call, &[reference(actor)], true)
                .is_err()
        );
        assert!(
            fixture
                .call(actor, call, &[Value::Null, Value::Vec3([0.0; 3])], true)
                .is_err()
        );
    }
    assert!(
        fixture
            .call(actor, "velocity", &[reference(actor)], false)
            .unwrap_err()
            .contains("no 3D physics")
    );
    assert!(
        fixture
            .call(actor, "sensor_entered", &[reference(actor)], true)
            .is_err()
    );
    fixture.world.get_mut(actor).unwrap().disabled = true;
    assert!(
        fixture
            .call(actor, "velocity", &[reference(actor)], true)
            .unwrap_err()
            .contains("active")
    );
    near(
        fixture.physics.world().linear_velocity(actor).unwrap(),
        [0.0; 3],
    );
    near(
        fixture.physics.world().angular_velocity(actor).unwrap(),
        [0.0; 3],
    );
    fixture.world.despawn_recursive(actor).unwrap();
    assert!(
        fixture
            .call(actor, "velocity", &[reference(actor)], true)
            .is_err()
    );
}

#[test]
fn queued_and_live_controls_obey_body_kind_and_rotation_lock() {
    let mut fixture = Fixture::new();
    let actor = fixture.actor("dynamic", false);
    fixture
        .call(
            actor,
            "set_velocity",
            &[reference(actor), Value::Vec3([1.0; 3])],
            true,
        )
        .unwrap();
    fixture
        .world
        .get_mut(actor)
        .unwrap()
        .components
        .get_mut(RigidBody3dComponent::TYPE_NAME)
        .unwrap()["lock_rotation"] = json!(true);
    fixture.step();
    fixture
        .call(
            actor,
            "set_angular_velocity",
            &[reference(actor), Value::Vec3([1.0; 3])],
            true,
        )
        .unwrap();
    near(
        fixture.physics.world().angular_velocity(actor).unwrap(),
        [0.0; 3],
    );
    fixture
        .world
        .get_mut(actor)
        .unwrap()
        .components
        .get_mut(RigidBody3dComponent::TYPE_NAME)
        .unwrap()["kind"] = json!("static");
    fixture.step();
    assert!(
        fixture
            .call(
                actor,
                "set_velocity",
                &[reference(actor), Value::Vec3([1.0; 3])],
                true
            )
            .is_err()
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
}

#[test]
fn two_scripts_observe_copied_sensor_events_without_draining_the_step() {
    let mut fixture = Fixture::new();
    let first = fixture.actor("dynamic", false);
    let second = fixture.actor("static", true);
    let mut sources = ScriptSources::new();
    sources.insert("events.decay", include_str!("physics3d_events.decay"));
    for actor in [first, second] {
        fixture.world.get_mut(actor).unwrap().components.insert(
            ScriptComponent::TYPE_NAME.into(),
            json!({"source": "events.decay", "script": "Events3d"}),
        );
    }
    let mut scripts = Scripts::new();
    fixture.step();
    let input = InputState::default();
    let (world, events) = fixture.physics.for_scripts();
    let report = scripts.advance(
        &mut fixture.world,
        fixture.extractor.components(),
        ScriptFrame::new(&sources, &input, 0.01).with_physics3d(Physics3d { world, events }),
    );
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    for actor in [first, second] {
        assert_eq!(scripts.field(actor, "count"), Some(&Value::Number(1.0)));
        assert_eq!(scripts.field(actor, "copied"), Some(&Value::Bool(true)));
    }
    fixture.world.get_mut(second).unwrap().disabled = true;
    assert_eq!(
        fixture.call(first, "sensor_entered", &[], true).unwrap(),
        Value::array(vec![])
    );
    fixture.step();
    assert_eq!(
        fixture.call(first, "sensor_entered", &[], true).unwrap(),
        Value::array(vec![])
    );
}

#[test]
fn solid_and_sensor_transition_queries_name_the_active_other_entity() {
    for (sensor, entered, exited) in [
        (false, "collision_started", "collision_stopped"),
        (true, "sensor_entered", "sensor_exited"),
    ] {
        let mut fixture = Fixture::new();
        let first = fixture.actor("dynamic", false);
        let second = fixture.actor("static", sensor);
        fixture
            .world
            .get_mut(second)
            .unwrap()
            .transform_3d
            .as_mut()
            .unwrap()
            .position = [0.8, 0.0, 0.0];
        fixture.step();
        assert_eq!(
            fixture.call(first, entered, &[], true).unwrap(),
            Value::array(vec![reference(second)])
        );
        assert_eq!(
            fixture.call(second, entered, &[], true).unwrap(),
            Value::array(vec![reference(first)])
        );
        fixture
            .world
            .get_mut(second)
            .unwrap()
            .transform_3d
            .as_mut()
            .unwrap()
            .position = [5.0, 0.0, 0.0];
        fixture.step();
        assert_eq!(
            fixture.call(first, exited, &[], true).unwrap(),
            Value::array(vec![reference(second)])
        );
        fixture.world.despawn_recursive(second).unwrap();
        assert_eq!(
            fixture.call(first, exited, &[], true).unwrap(),
            Value::array(vec![])
        );
    }
}

#[test]
fn velocity_kinematic_controls_work_and_impulses_require_dynamic_bodies() {
    let mut fixture = Fixture::new();
    let actor = fixture.actor("kinematic_velocity", false);
    fixture.step();
    fixture
        .call(
            actor,
            "set_velocity",
            &[reference(actor), Value::Vec3([1.0, 2.0, 3.0])],
            true,
        )
        .unwrap();
    fixture
        .call(
            actor,
            "set_angular_velocity",
            &[reference(actor), Value::Vec3([0.0, 0.0, 1.0])],
            true,
        )
        .unwrap();
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
        fixture.world.world_transform(actor).unwrap().position,
        [0.01, 0.02, 0.03],
    );
    near(
        fixture.physics.world().angular_velocity(actor).unwrap(),
        [0.0, 0.0, 1.0],
    );
}
