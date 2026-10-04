//! Command-backed authoring validates the policy and preserves live bodies on undo.
use serde_json::json;
use sindri_core::SceneComponent;
use sindri_core::{CommandBuffer, CommandHistory, EntityData, Transform3D, World, WorldCommand};
use sindri_scene::{OneWay2dComponent, SceneExtractor, ScenePhysics2d};
use std::time::Duration;

#[test]
fn authored_policy_validates_and_undo_preserves_velocity_and_joints() {
    let extractor = SceneExtractor::new().unwrap();
    let registry = extractor.components();
    let policy = OneWay2dComponent::TYPE_NAME;
    assert!(
        registry
            .validate_payload(policy, &json!({"normal": [0.0, 0.0]}))
            .is_err()
    );
    assert!(
        registry
            .validate_payload(policy, &json!({"angle": -0.1}))
            .is_err()
    );
    assert!(
        registry
            .validate_payload(policy, &json!({"angle": 2.0}))
            .is_err()
    );
    let mut world = World::default();
    let entity = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [
            (
                "sindri.physics2d.rigid_body".into(),
                registry
                    .default_payload("sindri.physics2d.rigid_body")
                    .unwrap()
                    .clone(),
            ),
            (
                "sindri.physics2d.collider".into(),
                registry
                    .default_payload("sindri.physics2d.collider")
                    .unwrap()
                    .clone(),
            ),
            (
                policy.into(),
                registry.default_payload(policy).unwrap().clone(),
            ),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let anchor = world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [10.0, 0.0, 0.0],
            ..Transform3D::default()
        }),
        components: [(
            "sindri.physics2d.collider".into(),
            registry
                .default_payload("sindri.physics2d.collider")
                .unwrap()
                .clone(),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut physics = ScenePhysics2d::top_down().unwrap();
    let step = Duration::from_nanos(16_666_667);
    physics.step(&mut world, registry, step).unwrap();
    physics
        .world_mut()
        .set_linear_velocity(entity, [3.0, 0.0])
        .unwrap();
    physics
        .world_mut()
        .connect_distance(entity, anchor, 100.0)
        .unwrap();
    let mut commands = CommandBuffer::new();
    commands.push(WorldCommand::SetComponent {
        entity,
        type_name: policy.into(),
        payload: json!({"normal": [1.0, 0.0], "angle": 0.5}),
    });
    let mut history = CommandHistory::default();
    history
        .apply(commands.into_transaction("Rotate support side"), &mut world)
        .unwrap();
    physics.step(&mut world, registry, step).unwrap();
    assert!((physics.world().linear_velocity(entity).unwrap()[0] - 3.0).abs() < 0.001);
    assert_eq!(physics.world().joint_count(), 1);
    history.undo(&mut world).unwrap();
    physics.step(&mut world, registry, step).unwrap();
    assert!((physics.world().linear_velocity(entity).unwrap()[0] - 3.0).abs() < 0.001);
    assert_eq!(physics.world().joint_count(), 1);
}
