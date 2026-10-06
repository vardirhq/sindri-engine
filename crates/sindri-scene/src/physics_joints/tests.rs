use std::time::Duration;

use serde_json::json;
use sindri_core::{
    CommandBuffer, CommandHistory, EntityData, EntityId, SceneComponent, SceneEntityId,
    Transform3D, World, WorldCommand,
};
use sindri_physics::{Collider2d, HingeSettings2d, RigidBody2d, RigidBodyKind};

use super::reference::resolve;
use super::{
    DistanceJoint2dComponent, HingeJoint2dComponent, SliderJoint2dComponent, SpringJoint2dComponent,
};
use crate::{
    Collider2dComponent, PhysicsSyncError, RigidBody2dComponent, SceneExtractor, ScenePhysics2d,
};

const STEP: Duration = Duration::from_millis(16);

fn body(world: &mut World, id: &str, position: [f32; 2], kind: RigidBodyKind) -> EntityId {
    world.spawn(EntityData {
        source_id: SceneEntityId::new(id).ok(),
        transform_3d: Some(Transform3D {
            position: [position[0], position[1], 0.0],
            ..Transform3D::default()
        }),
        components: [
            (
                Collider2dComponent::TYPE_NAME.into(),
                json!({"pieces": [Collider2d::circle(0.1)]}),
            ),
            (
                RigidBody2dComponent::TYPE_NAME.into(),
                serde_json::to_value(RigidBody2d {
                    kind,
                    ..RigidBody2d::default()
                })
                .unwrap(),
            ),
        ]
        .into(),
        ..EntityData::default()
    })
}

fn payload(distance: f32) -> serde_json::Value {
    json!({"first": "anchor", "second": "body", "max_distance": distance})
}

#[test]
fn edits_undo_rebuild_and_inactivity_preserve_joint_ownership() {
    let registry = SceneExtractor::new().unwrap().components().clone();
    let mut world = World::default();
    body(&mut world, "anchor", [0.0, 0.0], RigidBodyKind::Static);
    let moving = body(&mut world, "body", [1.0, 0.0], RigidBodyKind::Dynamic);
    let owner = world.spawn(EntityData {
        source_id: SceneEntityId::new("tether").ok(),
        components: [(DistanceJoint2dComponent::TYPE_NAME.into(), payload(2.0))].into(),
        ..EntityData::default()
    });
    let mut physics = ScenePhysics2d::top_down().unwrap();
    physics.step(&mut world, &registry, STEP).unwrap();
    for _ in 0..10 {
        physics.step(&mut world, &registry, STEP).unwrap();
    }
    assert_eq!(
        physics.world().joint_count(),
        1,
        "unchanged frames must not duplicate constraints"
    );
    physics
        .world_mut()
        .set_linear_velocity(moving, [3.0, 0.0])
        .unwrap();
    let mut commands = CommandBuffer::new();
    commands.push(WorldCommand::SetComponent {
        entity: owner,
        type_name: DistanceJoint2dComponent::TYPE_NAME.into(),
        payload: payload(3.0),
    });
    let mut history = CommandHistory::default();
    history
        .apply(commands.into_transaction("Lengthen tether"), &mut world)
        .unwrap();
    physics.step(&mut world, &registry, STEP).unwrap();
    history.undo(&mut world).unwrap();
    physics.step(&mut world, &registry, STEP).unwrap();
    assert!((physics.world().linear_velocity(moving).unwrap()[0] - 3.0).abs() < 1e-4);
    assert_eq!(physics.world().joint_count(), 1);
    world.get_mut(moving).unwrap().components.insert(
        Collider2dComponent::TYPE_NAME.into(),
        json!({"pieces": [Collider2d::circle(0.2)]}),
    );
    physics.step(&mut world, &registry, STEP).unwrap();
    assert_eq!(
        physics.world().joint_count(),
        1,
        "endpoint rebuild reconnects in the same fixed step"
    );
    world.get_mut(moving).unwrap().disabled = true;
    physics.step(&mut world, &registry, STEP).unwrap();
    assert_eq!(physics.world().joint_count(), 0);
    world.get_mut(moving).unwrap().disabled = false;
    physics.step(&mut world, &registry, STEP).unwrap();
    assert_eq!(physics.world().joint_count(), 1);
    world
        .get_mut(owner)
        .unwrap()
        .components
        .remove(DistanceJoint2dComponent::TYPE_NAME);
    physics.step(&mut world, &registry, STEP).unwrap();
    assert_eq!(physics.world().joint_count(), 0);
}

#[test]
fn references_prefer_the_local_namespace_and_cannot_cross_loaded_scene_roots() {
    let mut world = World::default();
    let root = world.spawn(EntityData::default());
    let other_root = world.spawn(EntityData::default());
    let owner = body(
        &mut world,
        "level/instance/tether",
        [0.0, 0.0],
        RigidBodyKind::Static,
    );
    let local = body(
        &mut world,
        "level/instance/anchor",
        [1.0, 0.0],
        RigidBodyKind::Static,
    );
    let external = body(&mut world, "anchor", [2.0, 0.0], RigidBodyKind::Static);
    world.set_parent(owner, Some(root)).unwrap();
    world.set_parent(local, Some(root)).unwrap();
    world.set_parent(external, Some(other_root)).unwrap();
    assert_eq!(resolve(&world, owner, "anchor"), Some(local));
    world.get_mut(local).unwrap().disabled = true;
    assert_eq!(resolve(&world, owner, "anchor"), None);
    assert_eq!(resolve(&world, owner, "missing"), None);
    assert_eq!(resolve(&world, owner, ""), None);
}

#[test]
fn hinge_motor_edits_undo_and_rebuild_preserve_ownership() {
    let registry = SceneExtractor::new().unwrap().components().clone();
    let mut world = World::default();
    body(&mut world, "anchor", [0.0, 0.0], RigidBodyKind::Static);
    let moving = body(&mut world, "body", [0.0, 0.0], RigidBodyKind::Dynamic);
    let component = HingeJoint2dComponent {
        first: "anchor".into(),
        second: "body".into(),
        settings: HingeSettings2d {
            motor_enabled: true,
            motor_velocity: 2.0,
            motor_max_torque: 1.0,
            ..HingeSettings2d::default()
        },
    };
    let payload = serde_json::to_value(&component).unwrap();
    registry
        .validate_payload(HingeJoint2dComponent::TYPE_NAME, &payload)
        .unwrap();
    let owner = world.spawn(EntityData {
        source_id: SceneEntityId::new("hinge").ok(),
        components: [(HingeJoint2dComponent::TYPE_NAME.into(), payload.clone())].into(),
        ..EntityData::default()
    });
    let mut physics = ScenePhysics2d::top_down().unwrap();
    for _ in 0..60 {
        physics.step(&mut world, &registry, STEP).unwrap();
    }
    assert!(physics.world().angular_velocity(moving).unwrap() > 1.5);
    let mut edited = payload;
    edited["motor_velocity"] = json!(-2.0);
    let mut commands = CommandBuffer::new();
    commands.push(WorldCommand::SetComponent {
        entity: owner,
        type_name: HingeJoint2dComponent::TYPE_NAME.into(),
        payload: edited,
    });
    let mut history = CommandHistory::default();
    history
        .apply(commands.into_transaction("Reverse hinge"), &mut world)
        .unwrap();
    for _ in 0..60 {
        physics.step(&mut world, &registry, STEP).unwrap();
    }
    assert!(physics.world().angular_velocity(moving).unwrap() < -1.5);
    history.undo(&mut world).unwrap();
    for _ in 0..60 {
        physics.step(&mut world, &registry, STEP).unwrap();
    }
    assert!(physics.world().angular_velocity(moving).unwrap() > 1.5);
    world.get_mut(moving).unwrap().components.insert(
        Collider2dComponent::TYPE_NAME.into(),
        json!({"pieces": [Collider2d::circle(0.2)]}),
    );
    physics.step(&mut world, &registry, STEP).unwrap();
    assert_eq!(physics.world().joint_count(), 1);
    world.get_mut(owner).unwrap().disabled = true;
    physics.step(&mut world, &registry, STEP).unwrap();
    assert_eq!(physics.world().joint_count(), 0);
    world.get_mut(owner).unwrap().disabled = false;
    physics.step(&mut world, &registry, STEP).unwrap();
    assert_eq!(physics.world().joint_count(), 1);
    world.get_mut(owner).unwrap().components.insert(
        DistanceJoint2dComponent::TYPE_NAME.into(),
        json!({"first": "anchor", "second": "body", "max_distance": 2.0}),
    );
    assert!(matches!(physics.step(&mut world, &registry, STEP),
        Err(PhysicsSyncError::ConflictingJointComponents(entity)) if entity == owner));
    assert_eq!(physics.world().joint_count(), 1);
    world
        .get_mut(owner)
        .unwrap()
        .components
        .remove(HingeJoint2dComponent::TYPE_NAME);
    physics.step(&mut world, &registry, STEP).unwrap();
    assert_eq!(
        physics.world().joint_count(),
        1,
        "switching kinds replaces ownership"
    );
}

#[test]
fn slider_and_spring_edits_undo_rebuild_and_suspend_without_duplicate_ownership() {
    for (kind, original, edited) in [
        (
            SliderJoint2dComponent::TYPE_NAME,
            json!({"first": "anchor", "second": "body", "motor_max_force": 1.0}),
            json!({"first": "anchor", "second": "body", "motor_max_force": 5.0}),
        ),
        (
            SpringJoint2dComponent::TYPE_NAME,
            json!({"first": "anchor", "second": "body", "rest_length": 1.0}),
            json!({"first": "anchor", "second": "body", "rest_length": 1.5}),
        ),
    ] {
        linear_lifecycle(kind, original, edited);
    }
}

fn linear_lifecycle(kind: &str, original: serde_json::Value, edited: serde_json::Value) {
    let registry = SceneExtractor::new().unwrap().components().clone();
    let mut world = World::default();
    body(&mut world, "anchor", [0.0, 0.0], RigidBodyKind::Static);
    let moving = body(&mut world, "body", [1.0, 0.0], RigidBodyKind::Dynamic);
    let owner = world.spawn(EntityData {
        source_id: SceneEntityId::new("joint").ok(),
        components: [(kind.into(), original)].into(),
        ..EntityData::default()
    });
    let mut physics = ScenePhysics2d::top_down().unwrap();
    physics.step(&mut world, &registry, STEP).unwrap();
    physics
        .world_mut()
        .set_linear_velocity(moving, [3.0, 0.0])
        .unwrap();
    let mut commands = CommandBuffer::new();
    commands.push(WorldCommand::SetComponent {
        entity: owner,
        type_name: kind.into(),
        payload: edited,
    });
    let mut history = CommandHistory::default();
    history
        .apply(commands.into_transaction("Edit linear joint"), &mut world)
        .unwrap();
    physics.step(&mut world, &registry, STEP).unwrap();
    assert!(physics.world().linear_velocity(moving).unwrap()[0] > 0.5);
    history.undo(&mut world).unwrap();
    physics.step(&mut world, &registry, STEP).unwrap();
    assert_eq!(physics.world().joint_count(), 1);
    world.get_mut(moving).unwrap().components.insert(
        Collider2dComponent::TYPE_NAME.into(),
        json!({"pieces": [Collider2d::circle(0.2)]}),
    );
    physics.step(&mut world, &registry, STEP).unwrap();
    assert_eq!(physics.world().joint_count(), 1);
    world.get_mut(moving).unwrap().disabled = true;
    physics.step(&mut world, &registry, STEP).unwrap();
    assert_eq!(physics.world().joint_count(), 0);
    world.get_mut(moving).unwrap().disabled = false;
    physics.step(&mut world, &registry, STEP).unwrap();
    assert_eq!(physics.world().joint_count(), 1);
    world.get_mut(owner).unwrap().disabled = true;
    physics.step(&mut world, &registry, STEP).unwrap();
    assert_eq!(physics.world().joint_count(), 0);
    world.get_mut(owner).unwrap().disabled = false;
    physics.step(&mut world, &registry, STEP).unwrap();
    assert_eq!(physics.world().joint_count(), 1);
    world.get_mut(owner).unwrap().components.remove(kind);
    physics.step(&mut world, &registry, STEP).unwrap();
    assert_eq!(physics.world().joint_count(), 0);
}
