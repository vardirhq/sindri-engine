//! The shared native/exported session offers the controller queue and snapshots.
use serde_json::json;
use sindri_causeway::{Session, extractor};
use sindri_core::{EntityData, SceneComponent, Transform3D, World};
use sindri_decay::{ScriptComponent, ScriptSources};
use sindri_platform::InputState;

#[test]
fn generic_session_applies_script_motion_and_reads_copied_ground_results() {
    let registry = extractor().unwrap();
    let collider = registry
        .components()
        .default_payload("sindri.physics2d.collider")
        .unwrap()
        .clone();
    let mut world = World::default();
    let mut floor = collider.clone();
    floor["pieces"][0]["shape"]["half_extents"] = json!([5.0, 0.5]);
    world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [("sindri.physics2d.collider".into(), floor)]
            .into_iter()
            .collect(),
        ..EntityData::default()
    });
    let actor = world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [0.0, 1.01, 0.0],
            ..Transform3D::default()
        }),
        components: [
            ("sindri.physics2d.collider".into(), collider),
            (
                "sindri.physics2d.character".into(),
                json!({"snap_distance": 0.2}),
            ),
            (
                ScriptComponent::TYPE_NAME.into(),
                json!({"source": "character_controller.decay", "script": "ControllerSession"}),
            ),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert(
        "character_controller.decay",
        include_str!("character_controller.decay"),
    );
    let mut session = Session::with_sources(registry.components().clone(), sources);
    for _ in 0..5 {
        session
            .step(
                &mut world,
                &InputState::default(),
                (960.0, 540.0),
                1.0 / 60.0,
            )
            .unwrap();
    }
    let position = world.world_transform(actor).unwrap().position;
    assert!(
        (position[0] - 0.02).abs() < 0.002,
        "queued once, then idle: {position:?}"
    );
    assert!((position[1] - 1.01).abs() < 0.002);
    assert!(
        (position[2] - 0.5).abs() < f32::EPSILON,
        "snapshot observed by script"
    );
}
