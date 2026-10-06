use decay_runtime::Value;
use serde_json::json;
use sindri_core::{EntityData, SceneComponent};
use sindri_scene::Character2dComponent;

use super::support::{Fixture, near, number, reference};

#[test]
fn typed_carry_fields_filter_inactive_support_without_losing_historical_motion() {
    let mut fixture = Fixture::new();
    let platform = fixture.solid("Platform", [0.0; 2], [5.0, 0.5]);
    let mut body = fixture
        .extractor
        .components()
        .default_payload("sindri.physics2d.rigid_body")
        .unwrap()
        .clone();
    body["kind"] = json!("kinematic_velocity");
    body["linear_velocity"] = json!([0.6, 0.0]);
    fixture
        .world
        .get_mut(platform)
        .unwrap()
        .components
        .insert("sindri.physics2d.rigid_body".into(), body);
    let actor = fixture.actor([0.0, 1.01]);
    let reader = fixture.world.spawn(EntityData::default());
    fixture.script(reader, "ControllerCarry", include_str!("carry.decay"));
    fixture.step();
    for active in [true, false] {
        fixture.world.get_mut(platform).unwrap().disabled = !active;
        let report = fixture.advance();
        assert!(report.failures.is_empty(), "{:?}", report.failures);
        near(number(fixture.field(reader, "carried")), 0.01);
        near(number(fixture.field(reader, "requested")), 0.01);
        assert_eq!(fixture.field(reader, "grounded"), &Value::Bool(active));
        assert_eq!(fixture.field(reader, "flags_clear"), &Value::Bool(true));
        let expected = if active {
            reference(platform)
        } else {
            Value::Null
        };
        assert_eq!(fixture.field(reader, "platform"), &expected);
        assert_eq!(fixture.field(reader, "ground"), &expected);
    }
    near(
        fixture.physics.character_motion(actor).unwrap().translation[0],
        0.01,
    );
}

#[test]
fn typed_slide_hits_filter_disabled_or_removed_colliders_and_are_copied() {
    let mut fixture = Fixture::new();
    fixture.solid("Floor", [0.0; 2], [5.0, 0.5]);
    let wall = fixture.solid("Wall", [1.5, 1.5], [0.5, 1.0]);
    let actor = fixture.actor([0.0, 1.01]);
    let reader = fixture.world.spawn(EntityData::default());
    fixture.script(reader, "ControllerCarry", include_str!("carry.decay"));
    fixture
        .physics
        .character_requests()
        .move_character(actor, [2.0, -0.1], true)
        .unwrap();
    fixture.step();
    let original = fixture.physics.character_motion(actor).unwrap().clone();
    assert!(
        original
            .slide
            .collisions
            .iter()
            .any(|hit| hit.entity == wall)
    );
    let report = fixture.advance();
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    assert!(number(fixture.field(reader, "collisions")) > 0.0);
    assert_eq!(fixture.physics.character_motion(actor).unwrap(), &original);
    fixture.world.get_mut(wall).unwrap().disabled = true;
    assert!(fixture.advance().failures.is_empty());
    let disabled_count = number(fixture.field(reader, "collisions"));
    assert!(
        disabled_count
            < number(&Value::Number(f64::from(
                u32::try_from(original.slide.collisions.len()).unwrap()
            )))
    );
    fixture.world.despawn_recursive(wall).unwrap();
    assert!(fixture.advance().failures.is_empty());
    near(number(fixture.field(reader, "collisions")), disabled_count);
}

#[test]
fn inactive_removed_component_and_unsynchronized_targets_return_null() {
    let mut fixture = Fixture::new();
    let actor = fixture.actor([0.0, 1.0]);
    assert_eq!(
        fixture
            .call(actor, "character_motion", &[reference(actor)], true)
            .unwrap(),
        Value::Null
    );
    fixture.step();
    assert!(matches!(
        fixture
            .call(actor, "character_motion", &[reference(actor)], true)
            .unwrap(),
        Value::Struct { .. }
    ));
    fixture.world.get_mut(actor).unwrap().disabled = true;
    assert_eq!(
        fixture
            .call(actor, "character_motion", &[reference(actor)], true)
            .unwrap(),
        Value::Null
    );
    fixture.world.get_mut(actor).unwrap().disabled = false;
    fixture
        .world
        .get_mut(actor)
        .unwrap()
        .components
        .remove(Character2dComponent::TYPE_NAME);
    assert_eq!(
        fixture
            .call(actor, "character_motion", &[reference(actor)], true)
            .unwrap(),
        Value::Null
    );
    fixture.world.despawn_recursive(actor).unwrap();
    assert!(
        fixture
            .call(actor, "character_motion", &[reference(actor)], true)
            .unwrap_err()
            .contains("no longer exists")
    );
}

#[test]
fn unguarded_optional_motion_access_is_a_typed_compile_error() {
    let mut fixture = Fixture::new();
    let actor = fixture.actor([0.0, 1.0]);
    fixture.script(actor, "UnsafeMotion", "script UnsafeMotion { fn update(dt: f32) { let motion = Physics.character_motion(this.entity); this.transform.position.z = motion.translation.x; } }");
    let failures = fixture.scripts.compile(
        &fixture.world,
        fixture.extractor.components(),
        &fixture.sources,
    );
    assert!(
        !failures.is_empty(),
        "optional motion requires a null check"
    );
}
