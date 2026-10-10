use decay_runtime::Value;
use serde_json::json;
use sindri_core::SceneComponent;
use sindri_scene::{Character3dComponent, RigidBody3dComponent};

use super::support::{Fixture, near, number, reference};

#[test]
fn scripts_queue_before_sync_and_observe_only_completed_motion() {
    let mut f = Fixture::new();
    let actor = f.actor([0.0; 3]);
    f.script(actor, "Queue3d", include_str!("queue.decay"));
    let report = f.advance();
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    near(number(f.field(actor, "observed")), -1.0);
    for part in f.world.world_transform(actor).unwrap().position {
        near(part, 0.0);
    }
    f.step();
    for (actual, expected) in f
        .world
        .world_transform(actor)
        .unwrap()
        .position
        .into_iter()
        .zip([1.0, 2.0, 3.0])
    {
        near(actual, expected);
    }
    assert!(f.advance().failures.is_empty());
    near(number(f.field(actor, "observed")), 3.0);
    f.step();
    near(
        f.physics.character_motion(actor).unwrap().translation[2],
        0.0,
    );
}

#[test]
fn invalid_requests_preserve_last_valid_input_and_missing_context_fails() {
    let mut f = Fixture::new();
    let actor = f.actor([0.0; 3]);
    let args = [
        reference(actor),
        Value::Vec3([1.0, 0.0, 2.0]),
        Value::Bool(false),
    ];
    f.call(actor, "move_character", &args, true).unwrap();
    for bad in [
        Value::Vec3([f64::NAN, 0.0, 0.0]),
        Value::Vec3([f64::INFINITY; 3]),
        Value::Vec3([f64::MAX; 3]),
        Value::Vec3([f64::from(f32::MAX); 3]),
        Value::Vec2([1.0; 2]),
    ] {
        assert!(
            f.call(
                actor,
                "move_character",
                &[reference(actor), bad, Value::Bool(false)],
                true
            )
            .is_err()
        );
    }
    assert!(
        f.call(
            actor,
            "move_character",
            &[reference(actor), args[1].clone(), Value::Number(1.0)],
            true
        )
        .is_err()
    );
    assert!(f.call(actor, "move_character", &args, false).is_err());
    assert!(
        f.call(actor, "character_motion", &[reference(actor)], false)
            .is_err()
    );
    assert!(f.call(actor, "move_character", &args[..2], true).is_err());
    assert!(f.call(actor, "character_motion", &args, true).is_err());
    f.step();
    near(f.world.world_transform(actor).unwrap().position[0], 1.0);
    near(f.world.world_transform(actor).unwrap().position[2], 2.0);
}

#[test]
fn inactive_invalid_settings_and_competing_bodies_are_rejected_before_queueing() {
    let mut f = Fixture::new();
    let actor = f.actor([0.0; 3]);
    let args = [
        reference(actor),
        Value::Vec3([2.0, 0.0, 0.0]),
        Value::Bool(false),
    ];
    f.world.get_mut(actor).unwrap().disabled = true;
    assert!(f.call(actor, "move_character", &args, true).is_err());
    f.world.get_mut(actor).unwrap().disabled = false;
    f.world.get_mut(actor).unwrap().components.insert(
        Character3dComponent::TYPE_NAME.into(),
        json!({"skin": -1.0}),
    );
    assert!(f.call(actor, "move_character", &args, true).is_err());
    f.world
        .get_mut(actor)
        .unwrap()
        .components
        .insert(Character3dComponent::TYPE_NAME.into(), json!({}));
    f.world
        .get_mut(actor)
        .unwrap()
        .components
        .insert(RigidBody3dComponent::TYPE_NAME.into(), json!({}));
    assert!(f.call(actor, "move_character", &args, true).is_err());
    f.world
        .get_mut(actor)
        .unwrap()
        .components
        .remove(RigidBody3dComponent::TYPE_NAME);
    f.step();
    near(
        f.physics.character_motion(actor).unwrap().translation[0],
        0.0,
    );
}
