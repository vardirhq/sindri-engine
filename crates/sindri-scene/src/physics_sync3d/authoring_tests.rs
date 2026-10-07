//! Registered defaults, checked component edits and reversible authoring.

use super::*;
use serde_json::json;
use sindri_core::{
    CommandBuffer, CommandHistory, EntityData, SceneComponent, SceneEntityId, WorldCommand,
};

#[test]
fn registered_3d_defaults_can_be_added_edited_undone_and_reopened() {
    let registry = crate::SceneExtractor::new().unwrap().components().clone();
    let mut world = World::default();
    let entity = world.spawn(EntityData {
        source_id: Some(SceneEntityId::new("body").unwrap()),
        ..EntityData::default()
    });
    let mut commands = CommandBuffer::new();
    for name in [
        RigidBody3dComponent::TYPE_NAME,
        Collider3dComponent::TYPE_NAME,
        PhysicsWorld3dComponent::TYPE_NAME,
    ] {
        let payload = registry.default_payload(name).unwrap().clone();
        registry.validate_payload(name, &payload).unwrap();
        commands.push(WorldCommand::SetComponent {
            entity,
            type_name: name.into(),
            payload,
        });
    }
    let mut history = CommandHistory::default();
    history
        .apply(commands.into_transaction("Add 3D physics"), &mut world)
        .unwrap();
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    super::tests::step(&mut physics, &mut world);
    assert!(physics.world().contains(entity));
    history.undo(&mut world).unwrap();
    super::tests::step(&mut physics, &mut world);
    assert!(physics.world().is_empty());
    history.redo(&mut world).unwrap();
    let mut edited = world.get(entity).unwrap().components[Collider3dComponent::TYPE_NAME].clone();
    edited["pieces"][0]["shape"] = json!({"shape": "sphere", "radius": 0.75});
    edited["future_authoring_field"] = json!("retained");
    registry
        .validate_payload(Collider3dComponent::TYPE_NAME, &edited)
        .unwrap();
    let mut commands = CommandBuffer::new();
    commands.push(WorldCommand::SetComponent {
        entity,
        type_name: Collider3dComponent::TYPE_NAME.into(),
        payload: edited.clone(),
    });
    history
        .apply(commands.into_transaction("Change 3D shape"), &mut world)
        .unwrap();
    super::tests::step(&mut physics, &mut world);
    history.undo(&mut world).unwrap();
    history.redo(&mut world).unwrap();
    assert_eq!(
        world.get(entity).unwrap().components[Collider3dComponent::TYPE_NAME],
        edited
    );
    let scene = world.to_scene().unwrap();
    let reopened = World::from_scene(&scene).unwrap().world;
    let payload =
        reopened.entities().next().unwrap().1.components[Collider3dComponent::TYPE_NAME].clone();
    assert_eq!(payload, edited);
    let mut reopened = reopened;
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    super::tests::step(&mut physics, &mut reopened);
    assert_eq!(physics.world().len(), 1);
}
