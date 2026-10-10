//! The typed host's 3D directions and methods, exercised through real scripts.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts, check_source};
use sindri_platform::InputState;

fn near(actual: [f32; 3], expected: [f32; 3]) {
    assert!(
        actual
            .into_iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() < 1.0e-4),
        "{actual:?} != {expected:?}"
    );
}

fn fixture(transform: Transform3D) -> (World, EntityId, ComponentSchemaRegistry) {
    let mut registry = ComponentSchemaRegistry::default();
    registry.register::<ScriptComponent>("Script").unwrap();
    let mut world = World::default();
    let entity = world.spawn(EntityData {
        name: Some("Actor".into()),
        transform_3d: Some(transform),
        components: [(
            ScriptComponent::TYPE_NAME.into(),
            json!({"source":"spatial.decay", "script":"Mover"}),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    (world, entity, registry)
}

fn run(world: &mut World, registry: &ComponentSchemaRegistry, body: &str) -> String {
    let mut sources = ScriptSources::new();
    sources.insert(
        "spatial.decay",
        format!("script Mover {{ fn update(dt: f32) {{ {body} }} }}"),
    );
    let result = Scripts::new().advance(
        world,
        registry,
        ScriptFrame::new(&sources, &InputState::default(), 0.1),
    );
    if result.is_quiet() {
        String::new()
    } else {
        format!("{result:?}")
    }
}

#[test]
fn a_parented_actor_aims_and_walks_in_world_space() {
    let (mut world, actor, registry) = fixture(Transform3D {
        position: [1.0, 2.0, 3.0],
        ..Transform3D::default()
    });
    let mut parent = Transform3D {
        position: [10.0, 5.0, -2.0],
        scale: [2.0, 3.0, 4.0],
        ..Transform3D::default()
    };
    parent.set_yaw_pitch_roll_radians([0.6, 0.2, -0.1]);
    let carrier = world.spawn(EntityData {
        transform_3d: Some(parent),
        ..EntityData::default()
    });
    world.set_parent(actor, Some(carrier)).unwrap();
    let start = world.world_transform(actor).unwrap().position;
    let failures = run(
        &mut world,
        &registry,
        r#"
        let actor = World.find("Actor");
        let target = actor.transform.world_position + Vec3(-3.0, 4.0, 0.0);
        actor.transform.look_at(target);
        this.transform.world_position += this.entity.transform.forward * 5.0;
    "#,
    );
    assert!(failures.is_empty(), "{failures}");
    near(
        world.world_transform(actor).unwrap().position,
        [start[0] - 3.0, start[1] + 4.0, start[2]],
    );
    near(
        world.world_transform(actor).unwrap().forward(),
        [-0.6, 0.8, 0.0],
    );
}

#[test]
fn directions_can_be_copied_but_not_written_through() {
    for target in [
        "this.transform.forward",
        "this.entity.transform.right",
        "other.transform.up",
    ] {
        for suffix in ["", ".x"] {
            let value = if suffix.is_empty() {
                "Vec3(0.0, 0.0, 1.0)"
            } else {
                "1.0"
            };
            let source = format!(
                "script Mover {{ fn update(dt: f32) {{ let other = World.find(\"Actor\"); {target}{suffix} = {value}; }} }}"
            );
            let checked = check_source(&source);
            assert!(!checked.compiles(), "{source}");
            assert!(
                checked
                    .diagnostics
                    .iter()
                    .any(|d| d.code == "immutable" && d.message.contains("read-only")),
                "{checked:?}"
            );
        }
    }
    let checked = check_source(
        "script Mover { fn update(dt: f32) { var facing = this.transform.forward; facing.x = 1.0; this.transform.position = facing; } }",
    );
    assert!(checked.compiles(), "{checked:?}");
}

#[test]
fn orbit_is_world_space_and_preserves_scale() {
    let (mut world, actor, registry) = fixture(Transform3D {
        position: [2.0, 0.0, 0.0],
        scale: [2.0, 3.0, 4.0],
        ..Transform3D::default()
    });
    let carrier = world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [10.0, 0.0, 0.0],
            scale: [2.0; 3],
            ..Transform3D::default()
        }),
        ..EntityData::default()
    });
    world.set_parent(actor, Some(carrier)).unwrap();
    let failures = run(
        &mut world,
        &registry,
        "this.entity.transform.rotate_around(Vec3(10.0, 0.0, 0.0), Vec3(0.0, 2.0, 0.0), PI / 2.0);",
    );
    assert!(failures.is_empty(), "{failures}");
    near(
        world.world_transform(actor).unwrap().position,
        [10.0, 0.0, -4.0],
    );
    near(
        world.get(actor).unwrap().transform_3d.unwrap().scale,
        [2.0, 3.0, 4.0],
    );
}

#[test]
fn invalid_operations_and_z_locked_orbits_preserve_the_transform() {
    for (locked, body) in [
        (
            false,
            "this.transform.look_at(this.transform.world_position);",
        ),
        (
            false,
            "this.transform.rotate_around(Vec3(0.0, 0.0, 0.0), Vec3(0.0, 0.0, 0.0), 1.0);",
        ),
        (false, "this.transform.look_at(Vec3(1e39, 0.0, 0.0));"),
        (
            true,
            "this.transform.rotate_around(Vec3(0.0, 0.0, 0.0), Vec3(0.0, 1.0, 0.0), 1.0);",
        ),
    ] {
        let original = Transform3D {
            position: [2.0, 0.0, 0.0],
            z_locked: locked,
            ..Transform3D::default()
        };
        let (mut world, actor, registry) = fixture(original);
        assert!(!run(&mut world, &registry, body).is_empty(), "{body}");
        assert_eq!(world.get(actor).unwrap().transform_3d, Some(original));
    }
}

#[test]
fn runtime_refuses_direction_writes_even_without_the_compiler() {
    use decay_ir::Path;
    use decay_runtime::{Host, Value};
    use sindri_decay::{
        AudioQueue, Blackboard, HostServices, PrefabSources, ProfileSources, ScriptContext,
        Spawning, WorldHost,
    };
    use std::collections::BTreeSet;
    let (mut world, actor, _) = fixture(Transform3D::default());
    let input = InputState::default();
    let mut board = Blackboard::default();
    let mut audio = AudioQueue::default();
    let profiles = ProfileSources::new();
    let started = BTreeSet::new();
    let mut spawned = Vec::new();
    let mut host = WorldHost::new(
        &mut world,
        actor,
        ScriptContext {
            input: &input,
            delta_seconds: 0.1,
            elapsed_seconds: 0.0,
        },
        &mut board,
        HostServices {
            spawning: Spawning {
                prefabs: PrefabSources::none(),
                started: &started,
                spawned: &mut spawned,
            },
            profiles: &profiles,
            audio: &mut audio,
            physics: None,
            screen_ui: None,
            aim: None,
            gestures: None,
            camera_pan: None,
            random: None,
            saves: None,
            effects: None,
            animations: None,
            scenes: None,
            tile_sets: None,
        },
    );
    for (subject, prefix) in [
        (None, vec!["this", "transform"]),
        (Some(actor.to_bits()), vec!["transform"]),
    ] {
        for (suffix, value) in [
            (vec!["forward"], Value::Vec3([1.0, 0.0, 0.0])),
            (vec!["right", "x"], Value::Number(2.0)),
        ] {
            let path = Path(
                prefix
                    .iter()
                    .chain(&suffix)
                    .map(|v| (*v).to_owned())
                    .collect(),
            );
            let error = host.store(subject, &path, value).unwrap_err();
            assert!(
                matches!(&error, decay_runtime::RuntimeError::Host(message) if message.contains("read-only")),
                "{error:?}"
            );
        }
    }
    for parts in [
        vec!["Pointer", "locked"],
        vec!["Input", "Pointer", "locked"],
    ] {
        let path = Path(parts.into_iter().map(str::to_owned).collect());
        let error = host.store(None, &path, Value::Bool(true)).unwrap_err();
        assert!(
            matches!(&error, decay_runtime::RuntimeError::Host(message) if message.contains("read-only")),
            "{error:?}"
        );
    }
    for prefix in [vec!["Pointer", "delta"], vec!["Input", "Pointer", "delta"]] {
        for (suffix, value) in [
            (vec![], Value::Vec2([1.0, 2.0])),
            (vec!["x"], Value::Number(2.0)),
        ] {
            let path = Path(
                prefix
                    .iter()
                    .chain(&suffix)
                    .map(|v| (*v).to_owned())
                    .collect(),
            );
            let error = host.store(None, &path, value).unwrap_err();
            assert!(
                matches!(&error, decay_runtime::RuntimeError::Host(message) if message.contains("read-only")),
                "{error:?}"
            );
        }
    }
    assert_eq!(
        world.get(actor).unwrap().transform_3d,
        Some(Transform3D::default())
    );
}

#[test]
fn aiming_preserves_a_locked_local_position_exactly_under_a_scaled_parent() {
    let original = Transform3D {
        position: [0.3, 0.7, 0.9],
        scale: [0.8, 0.9, 1.1],
        z_locked: true,
        ..Transform3D::default()
    };
    let (mut world, actor, registry) = fixture(original);
    let mut parent = Transform3D {
        position: [10.5, 20.7, 30.2],
        scale: [2.3, 3.4, 4.5],
        ..Transform3D::default()
    };
    parent.set_yaw_pitch_roll_radians([0.7, 0.2, 0.3]);
    let carrier = world.spawn(EntityData {
        transform_3d: Some(parent),
        ..EntityData::default()
    });
    world.set_parent(actor, Some(carrier)).unwrap();
    let failures = run(
        &mut world,
        &registry,
        "this.transform.look_at(this.transform.world_position + Vec3(0.0, 1.0, -1.0));",
    );
    assert!(failures.is_empty(), "{failures}");
    let actual = world.get(actor).unwrap().transform_3d.unwrap();
    assert_eq!(
        actual.position.map(f32::to_bits),
        original.position.map(f32::to_bits)
    );
    assert_eq!(
        actual.scale.map(f32::to_bits),
        original.scale.map(f32::to_bits)
    );
    let failures = run(
        &mut world,
        &registry,
        "this.transform.rotate_around(Vec3(0.0, 0.0, 0.0), Vec3(0.0, 1.0, 0.0), 0.0);",
    );
    assert!(failures.is_empty(), "{failures}");
    assert_eq!(world.get(actor).unwrap().transform_3d, Some(actual));
}

#[test]
fn an_orbit_under_a_singular_parent_is_refused_without_mutation() {
    let original = Transform3D {
        position: [2.0, 3.0, 4.0],
        ..Transform3D::default()
    };
    let (mut world, actor, registry) = fixture(original);
    let carrier = world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            scale: [0.0, 1.0, 1.0],
            ..Transform3D::default()
        }),
        ..EntityData::default()
    });
    world.set_parent(actor, Some(carrier)).unwrap();
    let failures = run(
        &mut world,
        &registry,
        "this.transform.rotate_around(Vec3(0.0, 0.0, 0.0), Vec3(0.0, 1.0, 0.0), 1.0);",
    );
    assert!(failures.contains("zero scale"), "{failures}");
    assert_eq!(world.get(actor).unwrap().transform_3d, Some(original));
}
