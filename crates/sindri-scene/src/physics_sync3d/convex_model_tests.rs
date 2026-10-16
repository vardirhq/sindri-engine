//! Scene hull lifecycle, scale, mixed compounds and atomic invalid authoring.
use super::tests::{components, near3, spawn, step};
use super::*;
use crate::{ModelConvexCollider3dComponent, ModelConvexGeometry3d};
use serde_json::json;
use sindri_core::{AssetId, EntityData, SceneComponent, Transform3D};
use sindri_physics::RaycastFilter3d;
use std::sync::Arc;

fn id() -> AssetId {
    AssetId::new("models/prop.glb").unwrap()
}
fn cube() -> Arc<ModelConvexGeometry3d> {
    Arc::new(
        ModelConvexGeometry3d::new(
            [-0.5, 0.5]
                .into_iter()
                .flat_map(|x| {
                    [-0.5, 0.5]
                        .into_iter()
                        .flat_map(move |y| [-0.5, 0.5].into_iter().map(move |z| [x, y, z]))
                })
                .collect(),
        )
        .unwrap(),
    )
}
fn prop(world: &mut World, position: [f32; 3], kind: RigidBodyKind) -> EntityId {
    world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position,
            ..Transform3D::default()
        }),
        components: [
            (
                ModelConvexCollider3dComponent::TYPE_NAME.into(),
                json!({"asset": id()}),
            ),
            (
                RigidBody3dComponent::TYPE_NAME.into(),
                json!(RigidBody3d {
                    kind,
                    ..RigidBody3d::default()
                }),
            ),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    })
}
fn ray(physics: &ScenePhysics3d, x: f32) -> Option<sindri_physics::RayHit3d> {
    physics
        .world()
        .raycast(
            [x, 10.0, 0.0],
            [0.0, -1.0, 0.0],
            20.0,
            RaycastFilter3d::default(),
        )
        .unwrap()
}

#[test]
fn scaled_dynamic_hull_lands_preserves_velocity_and_replays() {
    let mut world = World::default();
    spawn(
        &mut world,
        [0.0, -0.5, 0.0],
        None,
        Collider3d::cuboid([5.0, 0.5, 5.0]),
    );
    let actor = prop(&mut world, [0.0, 4.0, 0.0], RigidBodyKind::Dynamic);
    world
        .get_mut(actor)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .scale = [2.0, 3.0, 1.0];
    // Hold orientation fixed so the expected support height measures scale;
    // an unconstrained tall hull can tilt while settling on its contact points.
    world
        .get_mut(actor)
        .unwrap()
        .components
        .get_mut(RigidBody3dComponent::TYPE_NAME)
        .unwrap()["lock_rotation"] = json!(true);
    let mut physics = ScenePhysics3d::new([0.0, -9.81, 0.0]).unwrap();
    physics.bind_model_convex_collision(id(), cube());
    for _ in 0..200 {
        step(&mut physics, &mut world);
    }
    assert!(
        (world.world_transform(actor).unwrap().position[1] - 1.5).abs() < 0.03,
        "{:?}",
        world.world_transform(actor)
    );
    near3(world.world_transform(actor).unwrap().scale, [2.0, 3.0, 1.0]);
    let registration = physics.registered[&actor].clone();
    physics
        .world_mut()
        .set_linear_velocity(actor, [2.0, 3.0, 0.0])
        .unwrap();
    let mut replay = physics.clone();
    let mut replay_world = world.clone();
    for _ in 0..20 {
        step(&mut physics, &mut world);
        step(&mut replay, &mut replay_world);
        assert_eq!(
            world.world_transform(actor),
            replay_world.world_transform(actor)
        );
    }
    assert!(physics.world().linear_velocity(actor).unwrap()[0] > 1.9);
    assert!(registration == physics.registered[&actor]);
}

#[test]
fn inherited_pose_scaled_queries_mixed_pieces_and_lifecycle() {
    let mut world = World::default();
    let parent = world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [10.0, 0.0, 0.0],
            scale: [2.0, 3.0, 4.0],
            ..Transform3D::default()
        }),
        ..EntityData::default()
    });
    let actor = prop(&mut world, [0.0; 3], RigidBodyKind::Static);
    world.get_mut(actor).unwrap().parent = Some(parent);
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    physics.bind_model_convex_collision(id(), cube());
    step(&mut physics, &mut world);
    assert!((ray(&physics, 10.5).unwrap().point[1] - 1.5).abs() < 0.001);
    assert!(ray(&physics, 11.5).is_none());
    let angle = std::f32::consts::FRAC_PI_4;
    world
        .get_mut(parent)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .rotation = [0.0, angle.sin(), 0.0, angle.cos()];
    step(&mut physics, &mut world);
    assert!((ray(&physics, 11.5).unwrap().point[1] - 1.5).abs() < 0.001);
    world
        .get_mut(parent)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .rotation = [0.0, 0.0, 0.0, 1.0];
    world.get_mut(actor).unwrap().components.insert(
        Collider3dComponent::TYPE_NAME.into(),
        json!(Collider3d::sphere(1.0)),
    );
    // Nonuniform parent scale cannot represent a sphere; the old hull remains.
    assert!(
        physics
            .step(&mut world, &components(), Duration::from_millis(10))
            .is_err()
    );
    world.get_mut(actor).unwrap().components.insert(
        Collider3dComponent::TYPE_NAME.into(),
        json!(Collider3d::cuboid([1.0; 3])),
    );
    step(&mut physics, &mut world);
    assert!((ray(&physics, 10.5).unwrap().point[1] - 3.0).abs() < 0.001);
    world.get_mut(actor).unwrap().disabled = true;
    step(&mut physics, &mut world);
    assert!(!physics.world().contains(actor));
    world.get_mut(actor).unwrap().disabled = false;
    step(&mut physics, &mut world);
    assert!(physics.world().contains(actor));
    world
        .get_mut(actor)
        .unwrap()
        .components
        .remove(ModelConvexCollider3dComponent::TYPE_NAME);
    step(&mut physics, &mut world);
    assert!(physics.registered[&actor].convex.is_none());
}

#[test]
fn invalid_scale_ownership_properties_and_unbinding_leave_solver_unchanged() {
    let mut world = World::default();
    let actor = prop(&mut world, [0.0; 3], RigidBodyKind::Dynamic);
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    assert!(
        physics
            .step(&mut world, &components(), Duration::from_millis(10))
            .is_err()
    );
    physics.bind_model_convex_collision(id(), cube());
    step(&mut physics, &mut world);
    for scale in [[-1.0; 3], [0.0; 3], [f32::MAX; 3], [1.0e-20; 3]] {
        world
            .get_mut(actor)
            .unwrap()
            .transform_3d
            .as_mut()
            .unwrap()
            .scale = scale;
        assert!(
            physics
                .step(&mut world, &components(), Duration::from_millis(10))
                .is_err()
        );
        assert!((ray(&physics, 0.0).unwrap().point[1] - 0.5).abs() < 0.001);
    }
    world
        .get_mut(actor)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .scale = [1.0; 3];
    for (name, payload) in [
        (crate::Character3dComponent::TYPE_NAME, json!({})),
        (crate::VoxelCollider3dComponent::TYPE_NAME, json!({})),
        (
            crate::ModelCollider3dComponent::TYPE_NAME,
            json!({"asset": id()}),
        ),
        (
            crate::Collider2dComponent::TYPE_NAME,
            json!(sindri_physics::Collider2d::circle(0.5)),
        ),
    ] {
        world
            .get_mut(actor)
            .unwrap()
            .components
            .insert(name.into(), payload);
        assert!(
            physics
                .step(&mut world, &components(), Duration::from_millis(10))
                .is_err()
        );
        assert_eq!(ray(&physics, 0.0).unwrap().entity, actor);
        world.get_mut(actor).unwrap().components.remove(name);
    }
    world
        .get_mut(actor)
        .unwrap()
        .components
        .get_mut(ModelConvexCollider3dComponent::TYPE_NAME)
        .unwrap()["friction"] = json!(-1.0);
    assert!(
        physics
            .step(&mut world, &components(), Duration::from_millis(10))
            .is_err()
    );
    physics.unbind_model_convex_collision(&id());
    assert!(
        physics
            .step(&mut world, &components(), Duration::from_millis(10))
            .is_err()
    );
    assert_eq!(ray(&physics, 0.0).unwrap().entity, actor);
}

#[test]
fn hull_extent_drives_voxel_residency_and_z_lock_is_rejected() {
    let mut world = World::default();
    let actor = prop(&mut world, [0.0; 3], RigidBodyKind::Dynamic);
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    physics.bind_model_convex_collision(id(), cube());
    step(&mut physics, &mut world);
    let reaches = physics.reaches(&world, &physics.registered, [0.0; 3], 0.01);
    assert_eq!(reaches.len(), 1);
    assert!((reaches[0].reach - 0.75f32.sqrt()).abs() < 0.001);
    world
        .get_mut(actor)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .z_locked = true;
    assert!(matches!(
        physics.step(&mut world, &components(), Duration::from_millis(10)),
        Err(PhysicsSyncError::LockedDepth3d(_))
    ));
}

#[test]
fn hull_without_body_is_static_and_kinematic_velocity_uses_shared_controls() {
    let mut world = World::default();
    let actor = prop(&mut world, [0.0; 3], RigidBodyKind::Static);
    world
        .get_mut(actor)
        .unwrap()
        .components
        .remove(RigidBody3dComponent::TYPE_NAME);
    let mut physics = ScenePhysics3d::new([0.0, -9.81, 0.0]).unwrap();
    physics.bind_model_convex_collision(id(), cube());
    for _ in 0..10 {
        step(&mut physics, &mut world);
    }
    near3(world.world_transform(actor).unwrap().position, [0.0; 3]);
    world.get_mut(actor).unwrap().components.insert(
        RigidBody3dComponent::TYPE_NAME.into(),
        json!(RigidBody3d {
            kind: RigidBodyKind::KinematicVelocity,
            linear_velocity: [2.0, 0.0, 0.0],
            ..RigidBody3d::default()
        }),
    );
    for _ in 0..10 {
        step(&mut physics, &mut world);
    }
    near3(
        world.world_transform(actor).unwrap().position,
        [0.2, 0.0, 0.0],
    );
    let registry = components();
    assert!(
        registry
            .default_payload(ModelConvexCollider3dComponent::TYPE_NAME)
            .is_none()
    );
    assert!(
        registry
            .validate_payload(ModelConvexCollider3dComponent::TYPE_NAME, &json!({}))
            .is_err()
    );
}
