use decay_runtime::Value;
use serde_json::json;
use sindri_core::{EntityData, SceneComponent};
use sindri_scene::{Character3dComponent, RigidBody3dComponent};

use super::support::{Fixture, near, number, reference};

#[test]
fn typed_carry_support_and_nested_copies_preserve_cached_results() {
    let mut f = Fixture::new();
    let platform = f.solid("Platform", [0.0; 3], [5.0, 0.5, 5.0]);
    let mut body = f
        .extractor
        .components()
        .default_payload(RigidBody3dComponent::TYPE_NAME)
        .unwrap()
        .clone();
    body["kind"] = json!("kinematic_velocity");
    body["linear_velocity"] = json!([0.6, 0.0, 0.0]);
    f.world
        .get_mut(platform)
        .unwrap()
        .components
        .insert(RigidBody3dComponent::TYPE_NAME.into(), body);
    let actor = f.actor([0.0, 1.01, 0.0]);
    let reader = f.world.spawn(EntityData::default());
    f.script(reader, "Observe3d", include_str!("observe.decay"));
    f.step();
    let original = f.physics.character_motion(actor).unwrap().clone();
    for active in [true, false] {
        f.world.get_mut(platform).unwrap().disabled = !active;
        let report = f.advance();
        assert!(report.failures.is_empty(), "{:?}", report.failures);
        near(number(f.field(reader, "total")), 0.01);
        near(
            number(f.field(reader, "carried")),
            if active { 0.01 } else { 0.0 },
        );
        near(
            number(f.field(reader, "requested")),
            if active { 0.01 } else { 0.0 },
        );
        assert_eq!(f.field(reader, "grounded"), &Value::Bool(active));
        let expected = if active {
            reference(platform)
        } else {
            Value::Null
        };
        assert_eq!(f.field(reader, "platform"), &expected);
        assert_eq!(f.field(reader, "ground"), &expected);
        assert_eq!(f.physics.character_motion(actor), Some(&original));
    }
}

#[test]
fn collisions_filter_inactive_entities_and_unsynchronized_or_missing_actors_return_null() {
    let mut f = Fixture::new();
    let actor = f.actor([0.0; 3]);
    assert_eq!(
        f.call(actor, "character_motion", &[reference(actor)], true)
            .unwrap(),
        Value::Null
    );
    let wall = f.solid("Wall", [1.5, 0.0, 0.0], [0.5, 5.0, 5.0]);
    let reader = f.world.spawn(EntityData::default());
    f.script(reader, "Observe3d", include_str!("observe.decay"));
    f.call(
        actor,
        "move_character",
        &[
            reference(actor),
            Value::Vec3([2.0, 0.0, 0.0]),
            Value::Bool(false),
        ],
        true,
    )
    .unwrap();
    f.step();
    assert!(f.advance().failures.is_empty());
    assert!(number(f.field(reader, "collisions")) > 0.0);
    f.world.get_mut(wall).unwrap().disabled = true;
    assert!(f.advance().failures.is_empty());
    near(number(f.field(reader, "collisions")), 0.0);
    f.world.despawn_recursive(wall).unwrap();
    assert!(f.advance().failures.is_empty());
    f.world.get_mut(actor).unwrap().disabled = true;
    assert_eq!(
        f.call(actor, "character_motion", &[reference(actor)], true)
            .unwrap(),
        Value::Null
    );
    f.world.get_mut(actor).unwrap().disabled = false;
    f.world
        .get_mut(actor)
        .unwrap()
        .components
        .remove(Character3dComponent::TYPE_NAME);
    assert_eq!(
        f.call(actor, "character_motion", &[reference(actor)], true)
            .unwrap(),
        Value::Null
    );
}

#[test]
fn raw_predicted_contact_never_becomes_classified_support_in_decay() {
    let mut f = Fixture::new();
    f.solid("Floor", [0.0; 3], [5.0, 0.5, 5.0]);
    let actor = f.actor([0.0, 1.04, 0.0]);
    let reader = f.world.spawn(EntityData::default());
    f.script(reader, "Observe3d", include_str!("observe.decay"));
    f.step();
    let original = f.physics.character_motion(actor).unwrap();
    assert!(original.movement.grounded);
    assert!(!original.grounded);
    assert!(f.advance().failures.is_empty());
    assert_eq!(f.field(reader, "raw_grounded"), &Value::Bool(true));
    assert_eq!(f.field(reader, "grounded"), &Value::Bool(false));
    assert_eq!(f.field(reader, "ground"), &Value::Null);
}
