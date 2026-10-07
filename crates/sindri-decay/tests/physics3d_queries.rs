//! Typed queries use indexed 3D geometry and copied optional snapshots.

#[path = "physics3d_controls/support.rs"]
mod support;

use decay_runtime::Value;
use serde_json::json;
use sindri_core::{EntityData, EntityId, SceneComponent};
use sindri_decay::{Physics3d, ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_scene::{Collider3dComponent, PhysicsWorld3dComponent};
use support::{Fixture, near, reference};

fn placed(fixture: &mut Fixture, position: [f32; 3], sensor: bool) -> EntityId {
    let actor = fixture.actor("static", sensor);
    fixture
        .world
        .get_mut(actor)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position = position;
    actor
}

fn ray_args() -> Vec<Value> {
    vec![
        Value::Vec3([0.0; 3]),
        Value::Vec3([0.0, 0.0, 2.0]),
        Value::Number(10.0),
        Value::Number(f64::from(u32::MAX)),
        Value::Bool(false),
        Value::Null,
    ]
}

fn hit_entity(value: &Value) -> Value {
    let Value::Struct { shape, fields } = value else {
        panic!("expected hit: {value:?}")
    };
    assert_eq!(shape.name, "RayHit3d");
    fields[0].clone()
}

#[test]
fn ray_hit_has_xyz_normalized_distance_and_null_misses() {
    let mut fixture = Fixture::new();
    let target = placed(&mut fixture, [0.0, 0.0, 5.0], false);
    fixture.step();
    let hit = fixture.call(target, "raycast", &ray_args(), true).unwrap();
    assert_eq!(hit_entity(&hit), reference(target));
    let Value::Struct { fields, .. } = hit else {
        unreachable!()
    };
    assert_eq!(fields[1], Value::Vec3([0.0, 0.0, 4.5]));
    assert_eq!(fields[2], Value::Vec3([0.0, 0.0, -1.0]));
    assert_eq!(fields[3], Value::Number(4.5));
    let mut args = ray_args();
    args[2] = Value::Number(4.49);
    assert_eq!(
        fixture.call(target, "raycast", &args, true).unwrap(),
        Value::Null
    );
    args[2] = Value::Number(4.5);
    assert_eq!(
        hit_entity(&fixture.call(target, "raycast", &args, true).unwrap()),
        reference(target)
    );
    args[0] = Value::Vec3([0.0, 0.0, 5.0]);
    let Value::Struct { fields, .. } = fixture.call(target, "raycast", &args, true).unwrap() else {
        unreachable!()
    };
    assert_eq!(fields[2], Value::Vec3([0.0; 3]));
    assert_eq!(fields[3], Value::Number(0.0));
}

#[test]
fn masks_sensors_exclusion_and_unsynchronized_inactivity_filter_whole_entities() {
    let mut fixture = Fixture::new();
    let sensor = placed(&mut fixture, [0.0, 0.0, 2.0], true);
    let solid = placed(&mut fixture, [0.0, 0.0, 4.0], false);
    let farther = placed(&mut fixture, [0.0, 0.0, 6.0], false);
    for (entity, mask) in [(sensor, 1), (solid, 2), (farther, 2)] {
        fixture
            .world
            .get_mut(entity)
            .unwrap()
            .components
            .get_mut(Collider3dComponent::TYPE_NAME)
            .unwrap()["pieces"][0]["layers"]["memberships"] = json!(mask);
    }
    fixture.step();
    let mut args = ray_args();
    assert_eq!(
        hit_entity(&fixture.call(solid, "raycast", &args, true).unwrap()),
        reference(solid)
    );
    args[4] = Value::Bool(true);
    assert_eq!(
        hit_entity(&fixture.call(solid, "raycast", &args, true).unwrap()),
        reference(sensor)
    );
    args[3] = Value::Number(2.0);
    args[5] = reference(solid);
    assert_eq!(
        hit_entity(&fixture.call(solid, "raycast", &args, true).unwrap()),
        reference(farther)
    );
    args[5] = Value::Null;
    fixture.world.get_mut(solid).unwrap().disabled = true;
    assert_eq!(
        hit_entity(&fixture.call(sensor, "raycast", &args, true).unwrap()),
        reference(farther)
    );
    fixture.world.despawn_recursive(solid).unwrap();
    assert_eq!(
        hit_entity(&fixture.call(sensor, "raycast", &args, true).unwrap()),
        reference(farther)
    );
}

#[test]
fn overlap_lists_are_sorted_unique_and_casts_use_sphere_extent() {
    let mut fixture = Fixture::new();
    let first = placed(&mut fixture, [0.0, 0.0, 5.0], false);
    let second = placed(&mut fixture, [0.0, 0.0, 5.0], false);
    let payload = fixture
        .world
        .get_mut(first)
        .unwrap()
        .components
        .get_mut(Collider3dComponent::TYPE_NAME)
        .unwrap();
    let piece = payload["pieces"][0].clone();
    payload["pieces"].as_array_mut().unwrap().push(piece);
    fixture.step();
    let filter = [
        Value::Number(f64::from(u32::MAX)),
        Value::Bool(false),
        Value::Null,
    ];
    let mut args = vec![Value::Vec3([0.0, 0.0, 5.0]), Value::Number(1.0)];
    args.extend(filter.clone());
    assert_eq!(
        fixture.call(first, "overlap_sphere", &args, true).unwrap(),
        Value::array(vec![reference(first), reference(second)])
    );
    let mut cast = vec![
        Value::Vec3([0.0; 3]),
        Value::Number(0.5),
        Value::Vec3([0.0, 0.0, 10.0]),
        Value::Number(10.0),
    ];
    cast.extend(filter);
    let hit = fixture.call(first, "cast_sphere", &cast, true).unwrap();
    assert_eq!(hit_entity(&hit), reference(first));
    let Value::Struct { fields, .. } = hit else {
        unreachable!()
    };
    let Value::Number(distance) = fields[3] else {
        unreachable!()
    };
    assert!((distance - 4.0).abs() < 0.002);
    cast[6] = reference(first);
    assert_eq!(
        hit_entity(&fixture.call(first, "cast_sphere", &cast, true).unwrap()),
        reference(second)
    );
    args[4] = reference(first);
    assert_eq!(
        fixture.call(first, "overlap_sphere", &args, true).unwrap(),
        Value::array(vec![reference(second)])
    );
    cast[3] = Value::Number(0.0);
    assert_eq!(
        fixture.call(first, "cast_sphere", &cast, true).unwrap(),
        Value::Null
    );
    cast[3] = Value::Number(10.0);
    cast[6] = Value::Null;
    cast[0] = Value::Vec3([0.0, 0.0, 5.0]);
    let Value::Struct { fields, .. } = fixture.call(first, "cast_sphere", &cast, true).unwrap()
    else {
        unreachable!()
    };
    assert_eq!(fields[1], Value::Vec3([0.0, 0.0, 5.0]));
    assert_eq!(fields[2], Value::Vec3([0.0; 3]));
    assert_eq!(fields[3], Value::Number(0.0));
}

#[test]
fn invalid_query_values_arity_and_context_fail_without_changing_geometry() {
    let mut fixture = Fixture::new();
    let target = placed(&mut fixture, [0.0, 0.0, 5.0], false);
    fixture.step();
    let valid = ray_args();
    let expected = fixture.call(target, "raycast", &valid, true).unwrap();
    for (at, bad) in [
        (0, Value::Vec2([0.0; 2])),
        (0, Value::Vec3([f64::NAN; 3])),
        (1, Value::Vec3([0.0; 3])),
        (1, Value::Vec3([f64::MAX; 3])),
        (2, Value::Number(-1.0)),
        (2, Value::Number(f64::INFINITY)),
        (3, Value::Number(-1.0)),
        (3, Value::Number(0.5)),
        (3, Value::Number(f64::from(u32::MAX) + 1.0)),
        (4, Value::Number(1.0)),
        (5, Value::Number(1.0)),
    ] {
        let mut args = valid.clone();
        args[at] = bad;
        assert!(fixture.call(target, "raycast", &args, true).is_err());
    }
    assert!(fixture.call(target, "raycast", &valid[..5], true).is_err());
    assert!(
        fixture
            .call(target, "raycast", &valid, false)
            .unwrap_err()
            .contains("no 3D physics")
    );
    assert_eq!(
        fixture.call(target, "raycast", &valid, true).unwrap(),
        expected
    );
    near(
        fixture.world.world_transform(target).unwrap().position,
        [0.0, 0.0, 5.0],
    );
}

#[test]
fn typed_script_reads_optional_hit_and_mutates_only_its_copy() {
    let mut fixture = Fixture::new();
    let target = placed(&mut fixture, [0.0, 0.0, 5.0], false);
    let observer = fixture.world.spawn(EntityData::default());
    fixture.world.get_mut(observer).unwrap().components.insert(
        ScriptComponent::TYPE_NAME.into(),
        json!({"source":"query.decay", "script":"Query3d"}),
    );
    let mut sources = ScriptSources::new();
    sources.insert("query.decay", include_str!("physics3d_query.decay"));
    fixture.world.spawn(EntityData {
        components: [(
            PhysicsWorld3dComponent::TYPE_NAME.into(),
            json!({"layers": ["targets"]}),
        )]
        .into(),
        ..EntityData::default()
    });
    fixture.step();
    let input = InputState::default();
    let mut scripts = Scripts::new();
    let (world, events) = fixture.physics.for_scripts();
    let report = scripts.advance(
        &mut fixture.world,
        fixture.extractor.components(),
        ScriptFrame::new(&sources, &input, 0.01).with_physics3d(Physics3d { world, events }),
    );
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    assert_eq!(
        scripts.field(observer, "verified"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        hit_entity(
            &fixture
                .call(observer, "raycast", &ray_args(), true)
                .unwrap()
        ),
        reference(target)
    );
}

#[test]
fn sphere_queries_reject_invalid_dimensions_directions_and_arity() {
    let mut fixture = Fixture::new();
    let target = placed(&mut fixture, [0.0, 0.0, 5.0], false);
    fixture.step();
    let filter = [
        Value::Number(f64::from(u32::MAX)),
        Value::Bool(false),
        Value::Null,
    ];
    for radius in [-1.0, 0.0, f64::NAN, f64::INFINITY, f64::MAX] {
        let mut overlap = vec![Value::Vec3([0.0; 3]), Value::Number(radius)];
        overlap.extend(filter.clone());
        assert!(
            fixture
                .call(target, "overlap_sphere", &overlap, true)
                .is_err()
        );
        let mut cast = vec![
            Value::Vec3([0.0; 3]),
            Value::Number(radius),
            Value::Vec3([0.0, 0.0, 1.0]),
            Value::Number(10.0),
        ];
        cast.extend(filter.clone());
        assert!(fixture.call(target, "cast_sphere", &cast, true).is_err());
    }
    for direction in [
        Value::Vec3([0.0; 3]),
        Value::Vec3([f64::NAN; 3]),
        Value::Vec2([1.0; 2]),
    ] {
        let mut cast = vec![
            Value::Vec3([0.0; 3]),
            Value::Number(0.5),
            direction,
            Value::Number(10.0),
        ];
        cast.extend(filter.clone());
        assert!(fixture.call(target, "cast_sphere", &cast, true).is_err());
    }
    assert!(fixture.call(target, "overlap_sphere", &[], true).is_err());
    assert!(fixture.call(target, "cast_sphere", &[], true).is_err());
}
