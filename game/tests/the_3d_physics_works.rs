//! Shared native/exported session physics; this is not voxel/game proof.

use serde_json::json;
use sindri_causeway::{Session, extractor};
use sindri_core::{EntityData, EntityId, SceneComponent, Transform3D, World};
use sindri_decay::{ScriptComponent, ScriptSources};
use sindri_platform::InputState;
use sindri_scene::{Collider3dComponent, PhysicsWorld3dComponent, RigidBody3dComponent};

fn fixture() -> (World, Session, EntityId) {
    fixture_with_script(
        "Physics3dObserver",
        include_str!("physics3d_observer.decay"),
    )
}

fn fixture_with_script(script: &str, source: &str) -> (World, Session, EntityId) {
    let scene = extractor().unwrap();
    let registry = scene.components();
    let mut world = World::default();
    let mut collider = registry
        .default_payload(Collider3dComponent::TYPE_NAME)
        .unwrap()
        .clone();
    collider["pieces"][0]["shape"]["half_extents"] = json!([8.0, 0.25, 8.0]);
    collider["pieces"][0]["friction"] = json!(0.0);
    world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [(Collider3dComponent::TYPE_NAME.into(), collider)].into(),
        ..EntityData::default()
    });
    let mut settings = registry
        .default_payload(PhysicsWorld3dComponent::TYPE_NAME)
        .unwrap()
        .clone();
    settings["layers"] = json!(["floor"]);
    world.spawn(EntityData {
        components: [(PhysicsWorld3dComponent::TYPE_NAME.into(), settings)].into(),
        ..EntityData::default()
    });
    let mut body = registry
        .default_payload(RigidBody3dComponent::TYPE_NAME)
        .unwrap()
        .clone();
    body["linear_velocity"] = json!([0.2, 0.0, 0.3]);
    body["lock_rotation"] = json!(true);
    let actor = world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [2.0, 3.0, 3.0],
            ..Transform3D::default()
        }),
        components: [
            (
                Collider3dComponent::TYPE_NAME.into(),
                registry
                    .default_payload(Collider3dComponent::TYPE_NAME)
                    .unwrap()
                    .clone(),
            ),
            (RigidBody3dComponent::TYPE_NAME.into(), body),
            (
                ScriptComponent::TYPE_NAME.into(),
                json!({"source": "physics3d_observer.decay", "script": script}),
            ),
        ]
        .into(),
        ..EntityData::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert("physics3d_observer.decay", source);
    let session = Session::with_sources(registry.clone(), sources);
    (world, session, actor)
}

#[test]
fn shared_session_solves_xyz_motion_and_solid_landing_before_observer_scripts() {
    let (mut world, mut session, actor) = fixture();
    for _ in 0..200 {
        session
            .step(&mut world, &InputState::default(), (960.0, 540.0), 0.01)
            .unwrap();
    }
    let at = world.world_transform(actor).unwrap().position;
    assert!((at[0] - 2.4).abs() < 0.02, "{at:?}");
    assert!((at[1] - 0.75).abs() < 0.06, "{at:?}");
    assert!((at[2] - 3.6).abs() < 0.02, "{at:?}");
}

#[test]
fn shared_session_removes_disabled_3d_bodies_and_resumes_from_authored_motion() {
    let (mut world, mut session, actor) = fixture();
    session
        .step(&mut world, &InputState::default(), (960.0, 540.0), 0.01)
        .unwrap();
    world.get_mut(actor).unwrap().disabled = true;
    let held = world.world_transform(actor).unwrap().position;
    for _ in 0..10 {
        session
            .step(&mut world, &InputState::default(), (960.0, 540.0), 0.01)
            .unwrap();
    }
    for (actual, expected) in world
        .world_transform(actor)
        .unwrap()
        .position
        .into_iter()
        .zip(held)
    {
        assert!((actual - expected).abs() < f32::EPSILON);
    }
    world.get_mut(actor).unwrap().disabled = false;
    session
        .step(&mut world, &InputState::default(), (960.0, 540.0), 0.01)
        .unwrap();
    let at = world.world_transform(actor).unwrap().position;
    assert!(at[0] > held[0] && at[1] < held[1] && at[2] > held[2]);
}

#[test]
fn shared_session_offers_typed_3d_controls_and_completed_collision_events() {
    let (mut world, mut session, actor) =
        fixture_with_script("Physics3dDriver", include_str!("physics3d_driver.decay"));
    for _ in 0..200 {
        session
            .step(&mut world, &InputState::default(), (960.0, 540.0), 0.01)
            .unwrap();
    }
    let at = world.world_transform(actor).unwrap().position;
    assert!(at[0] > 2.9 && at[0] < 3.1, "{at:?}");
    assert!(at[1] > 0.7 && at[1] < 0.8, "{at:?}");
    assert!(at[2] > 4.3 && at[2] < 4.5, "{at:?}");
}
