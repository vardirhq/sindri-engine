//! Authored 3D scene lifecycle and transform synchronization.

use super::*;
use serde_json::json;
use sindri_core::{EntityData, SceneComponent, Transform3D};
use sindri_physics::{ColliderShape3d, PhysicsEventKind, RaycastFilter3d};

const STEP: Duration = Duration::from_millis(10);

fn components() -> ComponentSchemaRegistry {
    crate::SceneExtractor::new().unwrap().components().clone()
}
pub(super) fn spawn(
    world: &mut World,
    position: [f32; 3],
    kind: Option<RigidBodyKind>,
    collider: Collider3d,
) -> EntityId {
    let mut data = EntityData {
        transform_3d: Some(Transform3D {
            position,
            ..Transform3D::default()
        }),
        ..EntityData::default()
    };
    data.components.insert(
        Collider3dComponent::TYPE_NAME.into(),
        json!({"pieces": [collider]}),
    );
    if let Some(kind) = kind {
        data.components.insert(
            RigidBody3dComponent::TYPE_NAME.into(),
            json!(RigidBody3d {
                kind,
                position: [99.0; 3],
                ..RigidBody3d::default()
            }),
        );
    }
    world.spawn(data)
}
pub(super) fn near3(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 0.003, "{actual} != {expected}");
    }
}
pub(super) fn step(physics: &mut ScenePhysics3d, world: &mut World) {
    physics.step(world, &components(), STEP).unwrap();
}

#[test]
fn transform_starts_the_body_and_xyz_writeback_preserves_authored_data_and_scale() {
    let mut world = World::default();
    let entity = spawn(
        &mut world,
        [1.0, 2.0, 3.0],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.2),
    );
    world
        .get_mut(entity)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .scale = [2.0, 3.0, 4.0];
    let authored = world.get(entity).unwrap().components.clone();
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    step(&mut physics, &mut world);
    near3(
        physics.world().pose(entity).unwrap().position,
        [1.0, 2.0, 3.0],
    );
    physics
        .world_mut()
        .set_linear_velocity(entity, [1.0, 2.0, 3.0])
        .unwrap();
    physics
        .world_mut()
        .set_angular_velocity(entity, [0.0, 1.0, 0.0])
        .unwrap();
    for _ in 0..10 {
        step(&mut physics, &mut world);
    }
    let placed = world.world_transform(entity).unwrap();
    near3(placed.position, [1.1, 2.2, 3.3]);
    near3(placed.scale, [2.0, 3.0, 4.0]);
    assert!(placed.rotation[1] > 0.04);
    assert_eq!(world.get(entity).unwrap().components, authored);
    // Unchanged frames must retain the live solver velocity.
    near3(
        physics.world().linear_velocity(entity).unwrap(),
        [1.0, 2.0, 3.0],
    );
}

#[test]
fn scene_gravity_overrides_the_host_and_inactivity_restores_it() {
    let mut world = World::default();
    let entity = spawn(
        &mut world,
        [0.0; 3],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.2),
    );
    let mut settings = EntityData::default();
    settings.components.insert(
        PhysicsWorld3dComponent::TYPE_NAME.into(),
        json!({"gravity": [1.0, 2.0, 3.0], "layers": ["ground"]}),
    );
    let settings = world.spawn(settings);
    let mut physics = ScenePhysics3d::new([0.0, -5.0, 0.0]).unwrap();
    step(&mut physics, &mut world);
    near3(physics.world().gravity(), [1.0, 2.0, 3.0]);
    assert!(physics.world().linear_velocity(entity).unwrap()[2] > 0.0);
    world.get_mut(settings).unwrap().disabled = true;
    step(&mut physics, &mut world);
    near3(physics.world().gravity(), [0.0, -5.0, 0.0]);
}

#[test]
fn structural_edits_rebuild_and_transform_edits_preserve_velocity() {
    let mut world = World::default();
    let entity = spawn(
        &mut world,
        [0.0; 3],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.2),
    );
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    step(&mut physics, &mut world);
    physics
        .world_mut()
        .set_linear_velocity(entity, [1.0, 2.0, 3.0])
        .unwrap();
    world
        .get_mut(entity)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position = [5.0; 3];
    step(&mut physics, &mut world);
    near3(
        world.world_transform(entity).unwrap().position,
        [5.01, 5.02, 5.03],
    );
    near3(
        physics.world().linear_velocity(entity).unwrap(),
        [1.0, 2.0, 3.0],
    );
    world.get_mut(entity).unwrap().components.insert(
        Collider3dComponent::TYPE_NAME.into(),
        json!(Collider3d::cuboid([1.0; 3])),
    );
    step(&mut physics, &mut world);
    near3(physics.world().linear_velocity(entity).unwrap(), [0.0; 3]);
    let hits = physics
        .world()
        .overlap(
            ColliderShape3d::Sphere { radius: 0.1 },
            PhysicsPose3d {
                position: [5.8, 5.0, 5.0],
                ..PhysicsPose3d::default()
            },
            RaycastFilter3d::default(),
        )
        .unwrap();
    assert_eq!(hits, vec![entity]);
}

#[test]
fn collider_only_solids_and_compound_sensors_participate_in_scene_events() {
    let mut world = World::default();
    let floor = spawn(
        &mut world,
        [0.0; 3],
        None,
        Collider3d::cuboid([4.0, 0.25, 4.0]),
    );
    let actor = spawn(
        &mut world,
        [0.0, 2.0, 0.0],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.5),
    );
    let sensor = Collider3d {
        sensor: true,
        ..Collider3d::sphere(3.0)
    };
    let trigger = spawn(&mut world, [0.0; 3], None, sensor);
    world.get_mut(trigger).unwrap().components.insert(
        Collider3dComponent::TYPE_NAME.into(),
        json!({"pieces": [sensor, sensor]}),
    );
    let mut physics = ScenePhysics3d::new([0.0, -9.81, 0.0]).unwrap();
    let mut contact = false;
    let mut entered = false;
    for _ in 0..200 {
        step(&mut physics, &mut world);
        contact |= physics.events().iter().any(|event| {
            event.kind == PhysicsEventKind::CollisionStarted
                && [event.first, event.second].contains(&floor)
        });
        entered |= physics.events().iter().any(|event| {
            event.kind == PhysicsEventKind::SensorEntered
                && [event.first, event.second].contains(&trigger)
        });
    }
    assert!(contact && entered);
    assert!((world.world_transform(actor).unwrap().position[1] - 0.75).abs() < 0.06);
    assert_eq!(
        physics.world().body_kind(floor).unwrap(),
        RigidBodyKind::Static
    );
}

#[test]
fn inactivity_parent_inactivity_collider_removal_and_despawn_release_bodies() {
    let mut world = World::default();
    let parent = world.spawn(EntityData::default());
    let entity = spawn(
        &mut world,
        [0.0; 3],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.2),
    );
    world.set_parent(entity, Some(parent)).unwrap();
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    step(&mut physics, &mut world);
    assert!(physics.world().contains(entity));
    world.get_mut(parent).unwrap().disabled = true;
    step(&mut physics, &mut world);
    assert!(!physics.world().contains(entity));
    world.get_mut(parent).unwrap().disabled = false;
    step(&mut physics, &mut world);
    assert!(physics.world().contains(entity));
    world
        .get_mut(entity)
        .unwrap()
        .components
        .remove(Collider3dComponent::TYPE_NAME);
    step(&mut physics, &mut world);
    assert!(!physics.world().contains(entity));
    world.get_mut(entity).unwrap().components.insert(
        Collider3dComponent::TYPE_NAME.into(),
        json!(Collider3d::sphere(0.2)),
    );
    step(&mut physics, &mut world);
    assert!(physics.world().contains(entity));
    world.despawn_recursive(entity).unwrap();
    step(&mut physics, &mut world);
    assert!(physics.world().is_empty());
}

#[test]
fn kinematic_position_targets_write_back_and_authored_targets_take_effect_next_step() {
    let mut world = World::default();
    let entity = spawn(
        &mut world,
        [0.0; 3],
        Some(RigidBodyKind::KinematicPosition),
        Collider3d::sphere(0.2),
    );
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    step(&mut physics, &mut world);
    let target = PhysicsPose3d {
        position: [3.0, 4.0, 5.0],
        ..PhysicsPose3d::default()
    };
    physics
        .world_mut()
        .set_kinematic_target(entity, target)
        .unwrap();
    near3(world.world_transform(entity).unwrap().position, [0.0; 3]);
    step(&mut physics, &mut world);
    near3(
        world.world_transform(entity).unwrap().position,
        target.position,
    );
    world
        .get_mut(entity)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position = [6.0; 3];
    step(&mut physics, &mut world);
    near3(world.world_transform(entity).unwrap().position, [6.0; 3]);
}

#[test]
fn invalid_late_piece_or_batch_preserves_existing_bodies_and_gravity() {
    let mut world = World::default();
    let entity = spawn(
        &mut world,
        [0.0; 3],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.2),
    );
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    step(&mut physics, &mut world);
    physics
        .world_mut()
        .set_linear_velocity(entity, [1.0; 3])
        .unwrap();
    let bad = spawn(&mut world, [4.0; 3], None, Collider3d::sphere(-1.0));
    world.get_mut(bad).unwrap().components.insert(
        Collider3dComponent::TYPE_NAME.into(),
        json!({"pieces": [Collider3d::sphere(0.5), Collider3d::sphere(-1.0)]}),
    );
    let mut settings = EntityData::default();
    settings.components.insert(
        PhysicsWorld3dComponent::TYPE_NAME.into(),
        json!({"gravity": [1.0, 2.0, 3.0]}),
    );
    world.spawn(settings);
    assert!(matches!(
        physics.step(&mut world, &components(), STEP),
        Err(PhysicsSyncError::Physics(
            sindri_physics::PhysicsError::ColliderPiece { index: 1, .. }
        ))
    ));
    assert!(!physics.world().contains(bad));
    near3(physics.world().pose(entity).unwrap().position, [0.0; 3]);
    near3(physics.world().linear_velocity(entity).unwrap(), [1.0; 3]);
    world.get_mut(entity).unwrap().disabled = true;
    assert!(physics.step(&mut world, &components(), STEP).is_err());
    assert!(
        physics.world().contains(entity),
        "invalid batch must not apply removals"
    );
    near3(physics.world().gravity(), [0.0; 3]);
}

#[test]
fn both_drivers_refuse_mixed_dimension_ownership_before_mutation() {
    let mut world = World::default();
    let entity = spawn(&mut world, [0.0; 3], None, Collider3d::sphere(0.2));
    world
        .get_mut(entity)
        .unwrap()
        .components
        .insert(crate::RigidBody2dComponent::TYPE_NAME.into(), json!({}));
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    assert!(matches!(physics.step(&mut world, &components(), STEP),
        Err(PhysicsSyncError::ConflictingDimensions(found)) if found == entity));
    assert!(physics.world().is_empty());
    let mut flat = crate::ScenePhysics2d::top_down().unwrap();
    assert!(matches!(flat.step(&mut world, &components(), STEP),
        Err(PhysicsSyncError::ConflictingDimensions(found)) if found == entity));
}

#[test]
fn duplicate_worlds_and_moving_depth_locks_fail_before_registration() {
    let mut world = World::default();
    let entity = spawn(
        &mut world,
        [0.0; 3],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.2),
    );
    world
        .get_mut(entity)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .z_locked = true;
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    assert!(matches!(physics.step(&mut world, &components(), STEP),
        Err(PhysicsSyncError::LockedDepth3d(found)) if found == entity));
    world
        .get_mut(entity)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .z_locked = false;
    for _ in 0..2 {
        let mut settings = EntityData::default();
        settings
            .components
            .insert(PhysicsWorld3dComponent::TYPE_NAME.into(), json!({}));
        world.spawn(settings);
    }
    assert!(matches!(
        physics.step(&mut world, &components(), STEP),
        Err(PhysicsSyncError::MultipleWorlds3d)
    ));
    assert!(physics.world().is_empty());
}
