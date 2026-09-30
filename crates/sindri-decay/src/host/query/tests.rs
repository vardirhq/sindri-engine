//! Host errors and ordering, independently of the script compiler.

use std::collections::BTreeSet;

use decay_ir::Path;
use decay_runtime::{Host, RuntimeError, Value};
use serde_json::json;
use sindri_core::{EntityData, EntityId, SceneComponent, TagsComponent, Transform3D, World};
use sindri_platform::InputState;

use super::{QUERY_LIMIT, WorldHost};
use crate::{Blackboard, PrefabSources, ProfileSources, ScriptContext, Spawning};
use crate::host::WorldServices;

fn tagged(world: &mut World, tag: &str, position: [f32; 3]) -> EntityId {
    world.spawn(EntityData {
        transform_3d: Some(Transform3D { position, ..Transform3D::default() }),
        components: [(TagsComponent::TYPE_NAME.to_owned(), json!({ "tags": [tag] }))]
            .into_iter().collect(),
        ..EntityData::default()
    })
}

fn query(world: &mut World, name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
    let input = InputState::default();
    let prefabs = PrefabSources::new();
    let profiles = ProfileSources::new();
    let started = BTreeSet::new();
    let mut spawned = Vec::new();
    let mut board = Blackboard::default();
    let first = world.entities().next().map(|(id, _)| id);
    let entity = first.unwrap_or_else(|| world.spawn(EntityData::default()));
    let mut host = WorldHost::new(
        world, entity,
        ScriptContext { input: &input, delta_seconds: 0.0, elapsed_seconds: 0.0 },
        &mut board,
        WorldServices {
            spawning: Spawning { prefabs: &prefabs, started: &started, spawned: &mut spawned },
            profiles: &profiles,
            saves: None, effects: None, physics: None, screen_ui: None, aim: None,
            gestures: None, camera_pan: None, random: None, animations: None,
            scenes: None, tile_sets: None,
        },
    );
    host.call(None, &Path(vec!["World".to_owned(), name.to_owned()]), args)
        .map(|answer| answer.expect("registered World call"))
}

fn arguments() -> Vec<Value> {
    vec![Value::String("enemy".to_owned()), Value::Vec3([0.0; 3])]
}

fn nearest(world: &mut World) -> Value {
    query(world, "nearest", &arguments()).expect("query succeeds")
}

fn radius(world: &mut World, radius: f64) -> Vec<Value> {
    let mut args = arguments();
    args.push(Value::Number(radius));
    let answer = query(world, "within_radius", &args).expect("query succeeds");
    answer.elements().expect("a list").as_ref().clone()
}

fn reference(entity: EntityId) -> Value {
    Value::Reference(entity.to_bits())
}

#[test]
fn nearest_filters_tags_activity_and_missing_or_nonfinite_transforms() {
    let mut world = World::default();
    tagged(&mut world, "pickup", [0.0; 3]);
    let off = tagged(&mut world, "enemy", [0.1, 0.0, 0.0]);
    world.get_mut(off).expect("entity").disabled = true;
    let parent = world.spawn(EntityData { disabled: true, ..EntityData::default() });
    let child = tagged(&mut world, "enemy", [0.2, 0.0, 0.0]);
    world.set_parent(child, Some(parent)).expect("parent");
    let no_transform = tagged(&mut world, "enemy", [0.0; 3]);
    world.get_mut(no_transform).expect("entity").transform_3d = None;
    tagged(&mut world, "enemy", [f32::NAN, 0.0, 0.0]);
    let close = tagged(&mut world, "enemy", [0.0, 0.0, 2.0]);
    tagged(&mut world, "enemy", [3.0, 0.0, 0.0]);
    assert_eq!(nearest(&mut world), reference(close));
    assert_eq!(radius(&mut world, 2.0), vec![reference(close)]);
}

#[test]
fn no_usable_matches_means_null_and_an_empty_list() {
    let mut world = World::default();
    assert_eq!(nearest(&mut world), Value::Null);
    assert!(radius(&mut world, 5.0).is_empty());
    let enemy = tagged(&mut world, "enemy", [0.0; 3]);
    world.get_mut(enemy).expect("entity").transform_3d = None;
    tagged(&mut world, "friend", [0.0; 3]);
    assert_eq!(nearest(&mut world), Value::Null);
    assert!(radius(&mut world, 5.0).is_empty());
}

#[test]
fn the_supplied_world_position_is_the_query_origin() {
    let mut world = World::default();
    tagged(&mut world, "enemy", [1.0, 0.0, 0.0]);
    let near = tagged(&mut world, "enemy", [10.0, 0.0, 0.0]);
    let mut args = vec![Value::String("enemy".to_owned()), Value::Vec3([9.0, 0.0, 0.0])];
    assert_eq!(query(&mut world, "nearest", &args).expect("query"), reference(near));
    args.push(Value::Number(1.0));
    let found = query(&mut world, "within_radius", &args).expect("query");
    assert_eq!(found.elements().expect("list").as_slice(), &[reference(near)]);
}

#[test]
fn parent_translation_rotation_and_scale_determine_distance() {
    let mut world = World::default();
    let parent = world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [10.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 1.0, 0.0],
            scale: [2.0, 2.0, 2.0],
            ..Transform3D::default()
        }),
        ..EntityData::default()
    });
    // Local x=4 is world x=2: translating alone would put it at 14,
    // and ignoring the parent's scale would put it at 6.
    let child = tagged(&mut world, "enemy", [4.0, 0.0, 0.0]);
    world.set_parent(child, Some(parent)).expect("parent");
    tagged(&mut world, "enemy", [3.0, 0.0, 0.0]);
    assert_eq!(nearest(&mut world), reference(child));
    assert_eq!(radius(&mut world, 2.0), vec![reference(child)]);
}

#[test]
fn radius_is_inclusive_sorted_and_zero_includes_coincident_entities() {
    let mut world = World::default();
    let boundary = tagged(&mut world, "enemy", [3.0, 4.0, 0.0]);
    tagged(&mut world, "enemy", [0.0, 0.0, 5.01]);
    let near = tagged(&mut world, "enemy", [1.0, 0.0, 0.0]);
    let origin = tagged(&mut world, "enemy", [0.0; 3]);
    assert_eq!(radius(&mut world, 5.0), vec![reference(origin), reference(near), reference(boundary)]);
    assert_eq!(radius(&mut world, 0.0), vec![reference(origin)]);
}

#[test]
fn ties_follow_world_order_including_reused_slots() {
    let mut world = World::default();
    let old = tagged(&mut world, "enemy", [1.0, 0.0, 0.0]);
    tagged(&mut world, "enemy", [-1.0, 0.0, 0.0]);
    world.despawn_recursive(old).expect("despawn");
    tagged(&mut world, "enemy", [0.0, 1.0, 0.0]);
    let expected: Vec<_> = world.entities().map(|(id, _)| reference(id)).collect();
    for _ in 0..3 {
        assert_eq!(nearest(&mut world), expected[0]);
        assert_eq!(radius(&mut world, 1.0), expected);
    }
}

#[test]
fn large_coordinates_do_not_overflow_squared_distance() {
    let mut world = World::default();
    let farther = tagged(&mut world, "enemy", [3.0e30, 0.0, 0.0]);
    let nearer = tagged(&mut world, "enemy", [2.0e30, 0.0, 0.0]);
    assert_eq!(nearest(&mut world), reference(nearer));
    assert_eq!(radius(&mut world, f64::INFINITY), vec![reference(nearer), reference(farther)]);
    assert_eq!(radius(&mut world, 2.5e30), vec![reference(nearer)]);
}

#[test]
fn malformed_tags_keep_the_existing_host_error_contract() {
    let mut world = World::default();
    let entity = tagged(&mut world, "enemy", [0.0; 3]);
    world.get_mut(entity).expect("entity").components
        .insert(TagsComponent::TYPE_NAME.to_owned(), json!({ "tags": 7 }));
    for name in ["nearest", "within_radius"] {
        let mut args = arguments();
        if name == "within_radius" { args.push(Value::Number(1.0)); }
        let error = query(&mut world, name, &args).expect_err("bad authored tags").to_string();
        assert!(error.contains("could not be read"), "{error}");
    }
}

#[test]
fn invalid_arguments_name_the_call_and_the_problem() {
    let mut world = World::default();
    for (args, message) in [
        (vec![], "exactly 2 arguments"),
        (vec![Value::Number(1.0), Value::Vec3([0.0; 3])], "tag, as text"),
        (vec![Value::String("enemy".to_owned()), Value::Vec2([0.0; 2])], "Vec3"),
        (vec![Value::String("enemy".to_owned()), Value::Vec3([f64::NAN; 3])], "finite position"),
        (vec![Value::String("enemy".to_owned()), Value::Vec3([f64::INFINITY; 3])], "finite position"),
        (vec![Value::String("enemy".to_owned()), Value::Vec3([f64::MAX; 3])], "f32 range"),
    ] {
        let error = query(&mut world, "nearest", &args).expect_err("bad arguments").to_string();
        assert!(error.contains("World.nearest") && error.contains(message), "{error}");
    }
    for value in [Value::Number(-1.0), Value::Number(f64::NEG_INFINITY), Value::Number(f64::NAN), Value::Null] {
        let mut args = arguments();
        args.push(value);
        let error = query(&mut world, "within_radius", &args).expect_err("bad radius").to_string();
        assert!(error.contains("World.within_radius") && error.contains("radius"), "{error}");
    }
    let error = query(&mut world, "within_radius", &arguments()).expect_err("missing radius").to_string();
    assert!(error.contains("exactly 3 arguments"), "{error}");
    let mut args = arguments();
    args.push(Value::Number(1.0));
    assert!(query(&mut world, "nearest", &args).expect_err("extra argument").to_string().contains("exactly 2 arguments"));
}

#[test]
fn result_limit_counts_only_results_and_nearest_has_no_list_limit() {
    let mut world = World::default();
    for _ in 0..QUERY_LIMIT {
        tagged(&mut world, "enemy", [1.0, 0.0, 0.0]);
    }
    assert_eq!(radius(&mut world, 1.0).len(), QUERY_LIMIT);
    let outside = tagged(&mut world, "enemy", [2.0, 0.0, 0.0]);
    assert_eq!(radius(&mut world, 1.0).len(), QUERY_LIMIT);
    assert!(matches!(nearest(&mut world), Value::Reference(_)));
    let mut args = arguments();
    args.push(Value::Number(2.0));
    let error = query(&mut world, "within_radius", &args).expect_err("over limit").to_string();
    assert!(error.contains("more than 8192") && error.contains("World.within_radius"), "{error}");
    world.get_mut(outside).expect("entity").transform_3d = None;
    assert_eq!(radius(&mut world, f64::INFINITY).len(), QUERY_LIMIT);
}
