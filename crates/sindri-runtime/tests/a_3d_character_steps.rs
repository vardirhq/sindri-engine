//! Scene-owned character movement uses the same Session and replay as every host.
use serde_json::json;
use sindri_core::{EntityData, SceneComponent, Transform3D, World};
use sindri_decay::{ScriptComponent, ScriptSources};
use sindri_platform::InputState;
use sindri_runtime::Session;
use sindri_scene::{Character3dComponent, Collider3dComponent, SceneExtractor};

#[test]
fn queued_character_motion_steps_once_and_checkpoint_replays_it() {
    let mut extractor = SceneExtractor::new().unwrap();
    extractor.register::<ScriptComponent>("Script").unwrap();
    let mut session = Session::with_sources(extractor.components().clone(), ScriptSources::new());
    let mut world = World::default();
    let actor = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [
            (Character3dComponent::TYPE_NAME.into(), json!({})),
            (
                Collider3dComponent::TYPE_NAME.into(),
                extractor
                    .components()
                    .default_payload(Collider3dComponent::TYPE_NAME)
                    .unwrap()
                    .clone(),
            ),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    session
        .character_requests3d()
        .move_character(actor, [1.0, 2.0, 3.0], false)
        .unwrap();
    let checkpoint = session.checkpoint();
    let initial = world.clone();
    let input = InputState::default();
    session
        .step(&mut world, &input, (640.0, 360.0), 0.01)
        .unwrap();
    let first = world.world_transform(actor).unwrap();
    let motion = session.physics3d().character_motion(actor).unwrap().clone();
    for (a, b) in first.position.into_iter().zip([1.0, 2.0, 3.0]) {
        assert!((a - b).abs() < 0.001);
    }
    session
        .step(&mut world, &input, (640.0, 360.0), 0.01)
        .unwrap();
    assert_eq!(world.world_transform(actor), Some(first));
    session.restore(&checkpoint);
    world = initial;
    session
        .step(&mut world, &input, (640.0, 360.0), 0.01)
        .unwrap();
    assert_eq!(world.world_transform(actor), Some(first));
    assert_eq!(session.physics3d().character_motion(actor), Some(&motion));
}

#[test]
fn shared_session_carries_from_first_solve_and_checkpoints_replay_support() {
    let mut extractor = SceneExtractor::new().unwrap();
    extractor.register::<ScriptComponent>("Script").unwrap();
    let mut session = Session::with_sources(extractor.components().clone(), ScriptSources::new());
    let mut world = World::default();
    let collider = extractor
        .components()
        .default_payload(Collider3dComponent::TYPE_NAME)
        .unwrap()
        .clone();
    let mut platform_collider = collider.clone();
    platform_collider["pieces"][0]["shape"]["half_extents"] = json!([5.0, 0.5, 5.0]);
    let mut body = extractor
        .components()
        .default_payload(sindri_scene::RigidBody3dComponent::TYPE_NAME)
        .unwrap()
        .clone();
    body["kind"] = json!("kinematic_velocity");
    body["linear_velocity"] = json!([5.0, 0.0, 0.0]);
    let platform = world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [0.0, -0.5, 0.0],
            ..Transform3D::default()
        }),
        components: [
            (Collider3dComponent::TYPE_NAME.into(), platform_collider),
            (sindri_scene::RigidBody3dComponent::TYPE_NAME.into(), body),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let actor = world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [0.0, 0.51, 0.0],
            ..Transform3D::default()
        }),
        components: [
            (Character3dComponent::TYPE_NAME.into(), json!({})),
            (Collider3dComponent::TYPE_NAME.into(), collider),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let input = InputState::default();
    session
        .step(&mut world, &input, (640.0, 360.0), 0.01)
        .unwrap();
    assert!((world.world_transform(actor).unwrap().position[0] - 0.05).abs() < 0.001);
    assert_eq!(
        session
            .physics3d()
            .character_motion(actor)
            .unwrap()
            .platform
            .as_ref()
            .unwrap()
            .entity,
        platform
    );
    let checkpoint = session.checkpoint();
    let saved = world.clone();
    session
        .step(&mut world, &input, (640.0, 360.0), 0.01)
        .unwrap();
    let result = session.physics3d().character_motion(actor).unwrap().clone();
    let position = world.world_transform(actor).unwrap();
    session.restore(&checkpoint);
    world = saved;
    session
        .step(&mut world, &input, (640.0, 360.0), 0.01)
        .unwrap();
    assert_eq!(world.world_transform(actor), Some(position));
    assert_eq!(session.physics3d().character_motion(actor), Some(&result));
}

#[test]
fn decay_requests_use_shared_session_order_and_checkpoint_replay() {
    let mut extractor = SceneExtractor::new().unwrap();
    extractor.register::<ScriptComponent>("Script").unwrap();
    let mut sources = ScriptSources::new();
    sources.insert(
        "queue.decay",
        include_str!("../../sindri-decay/tests/character_controllers3d/queue.decay"),
    );
    let mut session = Session::with_sources(extractor.components().clone(), sources);
    let mut world = World::default();
    let actor = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [
            (Character3dComponent::TYPE_NAME.into(), json!({})),
            (
                Collider3dComponent::TYPE_NAME.into(),
                extractor
                    .components()
                    .default_payload(Collider3dComponent::TYPE_NAME)
                    .unwrap()
                    .clone(),
            ),
            (
                ScriptComponent::TYPE_NAME.into(),
                json!({"source":"queue.decay", "script":"Queue3d"}),
            ),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let input = InputState::default();
    let report = session
        .step(&mut world, &input, (640.0, 360.0), 0.01)
        .unwrap();
    assert!(
        report.scripts.failures.is_empty(),
        "{:?}",
        report.scripts.failures
    );
    assert!(
        world
            .world_transform(actor)
            .unwrap()
            .position
            .into_iter()
            .all(|part| part.abs() < f32::EPSILON)
    );
    let checkpoint = session.checkpoint();
    let saved = world.clone();
    session
        .step(&mut world, &input, (640.0, 360.0), 0.01)
        .unwrap();
    let moved = world.world_transform(actor).unwrap();
    for (actual, expected) in moved.position.into_iter().zip([1.0, 2.0, 3.0]) {
        assert!((actual - expected).abs() < 0.001);
    }
    assert_eq!(
        session.scripts().field(actor, "observed"),
        Some(&sindri_decay::ScriptValue::Number(3.0))
    );
    session.restore(&checkpoint);
    world = saved;
    session
        .step(&mut world, &input, (640.0, 360.0), 0.01)
        .unwrap();
    assert_eq!(world.world_transform(actor), Some(moved));
    assert_eq!(
        session.scripts().field(actor, "observed"),
        Some(&sindri_decay::ScriptValue::Number(3.0))
    );
}
