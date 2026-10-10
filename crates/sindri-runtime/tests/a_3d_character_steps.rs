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
