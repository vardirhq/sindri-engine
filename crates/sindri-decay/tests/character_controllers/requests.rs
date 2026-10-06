use decay_runtime::Value;
use serde_json::json;
use sindri_core::SceneComponent;
use sindri_scene::Character2dComponent;

use super::support::{Fixture, near, number, reference};

#[test]
fn typed_requests_replace_input_and_read_previous_copied_motion() {
    let mut fixture = Fixture::new();
    fixture.solid("Floor", [0.0; 2], [5.0, 0.5]);
    let actor = fixture.actor([0.0, 1.01]);
    fixture.script(actor, "ControllerMotion", include_str!("motion.decay"));
    assert!(fixture.advance().failures.is_empty());
    assert_eq!(fixture.field(actor, "missing"), &Value::Bool(true));
    near(
        fixture.world.world_transform(actor).unwrap().position[0],
        0.0,
    );
    assert!(fixture.physics.character_motion(actor).is_none());
    fixture.step();
    near(
        fixture.world.world_transform(actor).unwrap().position[0],
        0.25,
    );
    assert!(fixture.advance().failures.is_empty());
    near(number(fixture.field(actor, "x")), 0.25);
    assert_eq!(fixture.field(actor, "grounded"), &Value::Bool(true));
    assert_eq!(fixture.field(actor, "copied"), &Value::Bool(true));
    near(
        fixture.physics.character_motion(actor).unwrap().translation[0],
        0.25,
    );
    fixture.step();
    near(
        fixture.world.world_transform(actor).unwrap().position[0],
        0.5,
    );
    fixture.step();
    near(
        fixture.world.world_transform(actor).unwrap().position[0],
        0.5,
    );
}

#[test]
fn invalid_displacements_and_argument_types_preserve_pending_motion() {
    let mut fixture = Fixture::new();
    let actor = fixture.actor([0.0, 2.0]);
    let valid = [
        reference(actor),
        Value::Vec2([0.3, 0.0]),
        Value::Bool(false),
    ];
    fixture.call(actor, "move_character", &valid, true).unwrap();
    for bad in [
        Value::Vec2([f64::NAN, 0.0]),
        Value::Vec2([f64::MAX, 0.0]),
        Value::Vec2([f64::from(f32::MAX); 2]),
        Value::Number(0.0),
    ] {
        assert!(
            fixture
                .call(
                    actor,
                    "move_character",
                    &[reference(actor), bad, Value::Bool(false)],
                    true
                )
                .is_err()
        );
    }
    assert!(
        fixture
            .call(
                actor,
                "move_character",
                &[
                    reference(actor),
                    Value::Vec2([1.0, 0.0]),
                    Value::Number(0.0)
                ],
                true
            )
            .is_err()
    );
    fixture.step();
    near(
        fixture.world.world_transform(actor).unwrap().position[0],
        0.3,
    );
}

#[test]
fn requests_reject_missing_context_invalid_authorship_and_inactive_entities() {
    let mut fixture = Fixture::new();
    let actor = fixture.actor([0.0, 2.0]);
    let args = [
        reference(actor),
        Value::Vec2([0.3, 0.0]),
        Value::Bool(false),
    ];
    assert!(
        fixture
            .call(actor, "move_character", &args, false)
            .unwrap_err()
            .contains("scene character controllers")
    );
    assert!(
        fixture
            .call(actor, "character_motion", &[reference(actor)], false)
            .is_err()
    );
    fixture.world.get_mut(actor).unwrap().disabled = true;
    assert!(
        fixture
            .call(actor, "move_character", &args, true)
            .unwrap_err()
            .contains("active")
    );
    fixture.world.get_mut(actor).unwrap().disabled = false;
    fixture
        .world
        .get_mut(actor)
        .unwrap()
        .components
        .insert("sindri.physics2d.rigid_body".into(), json!({}));
    assert!(
        fixture
            .call(actor, "move_character", &args, true)
            .unwrap_err()
            .contains("owns its body")
    );
    fixture
        .world
        .get_mut(actor)
        .unwrap()
        .components
        .remove("sindri.physics2d.rigid_body");
    fixture.world.get_mut(actor).unwrap().components.insert(
        Character2dComponent::TYPE_NAME.into(),
        json!({"skin": -1.0}),
    );
    assert!(
        fixture
            .call(actor, "move_character", &args, true)
            .unwrap_err()
            .contains("invalid character")
    );
    fixture
        .world
        .get_mut(actor)
        .unwrap()
        .components
        .insert(Character2dComponent::TYPE_NAME.into(), json!({}));
    fixture.world.get_mut(actor).unwrap().transform_3d = None;
    assert!(
        fixture
            .call(actor, "move_character", &args, true)
            .unwrap_err()
            .contains("requires a transform")
    );
    fixture
        .world
        .get_mut(actor)
        .unwrap()
        .components
        .remove(Character2dComponent::TYPE_NAME);
    assert!(
        fixture
            .call(actor, "move_character", &args, true)
            .unwrap_err()
            .contains("no authored Character")
    );
    assert!(
        fixture
            .call(actor, "move_character", &[Value::Null], true)
            .is_err()
    );
}

#[test]
fn typed_drop_through_bypasses_one_way_support() {
    let mut fixture = Fixture::new();
    let floor = fixture.solid("Floor", [0.0; 2], [5.0, 0.5]);
    fixture.world.get_mut(floor).unwrap().components.insert(
        "sindri.physics2d.one_way".into(),
        json!({"normal": [0.0, 1.0], "angle": 0.8}),
    );
    let actor = fixture.actor([0.0, 1.01]);
    fixture.step();
    assert!(fixture.physics.character_motion(actor).unwrap().grounded);
    fixture.script(actor, "ControllerDrop", include_str!("drop.decay"));
    let report = fixture.advance();
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    fixture.step();
    near(
        fixture.world.world_transform(actor).unwrap().position[1],
        0.71,
    );
    assert!(!fixture.physics.character_motion(actor).unwrap().grounded);
}

#[test]
fn drop_cancellation_and_invalid_durations_preserve_the_controller_timer() {
    for cancel in [true, false] {
        let mut fixture = Fixture::new();
        let floor = fixture.solid("Floor", [0.0; 2], [5.0, 0.5]);
        fixture.world.get_mut(floor).unwrap().components.insert(
            "sindri.physics2d.one_way".into(),
            json!({"normal": [0.0, 1.0], "angle": 0.8}),
        );
        let actor = fixture.actor([0.0, 1.01]);
        fixture
            .call(
                actor,
                "drop_through",
                &[reference(actor), Value::Number(1.0)],
                true,
            )
            .unwrap();
        for invalid in [-1.0, f64::NAN, f64::INFINITY, f64::MAX] {
            assert!(
                fixture
                    .call(
                        actor,
                        "drop_through",
                        &[reference(actor), Value::Number(invalid)],
                        true
                    )
                    .is_err()
            );
        }
        if cancel {
            fixture
                .call(
                    actor,
                    "drop_through",
                    &[reference(actor), Value::Number(0.0)],
                    true,
                )
                .unwrap();
        }
        fixture
            .call(
                actor,
                "move_character",
                &[
                    reference(actor),
                    Value::Vec2([0.0, -0.3]),
                    Value::Bool(false),
                ],
                true,
            )
            .unwrap();
        fixture.step();
        near(
            fixture.world.world_transform(actor).unwrap().position[1],
            if cancel { 1.01 } else { 0.71 },
        );
    }
}

#[test]
fn dynamic_drop_through_retains_physics_only_host_compatibility() {
    let mut fixture = Fixture::new();
    let body = fixture.solid("Dynamic", [0.0, 1.0], [0.5; 2]);
    let payload = fixture
        .extractor
        .components()
        .default_payload("sindri.physics2d.rigid_body")
        .unwrap()
        .clone();
    fixture
        .world
        .get_mut(body)
        .unwrap()
        .components
        .insert("sindri.physics2d.rigid_body".into(), payload);
    fixture
        .call(
            body,
            "drop_through",
            &[reference(body), Value::Number(0.5)],
            false,
        )
        .unwrap();
    fixture.step();
    fixture
        .call(
            body,
            "drop_through",
            &[reference(body), Value::Number(0.0)],
            false,
        )
        .unwrap();
}
