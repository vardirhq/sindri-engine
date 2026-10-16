//! Profile precedence and atomic coefficient-only scene synchronization.
use super::tests::{components, near3, spawn, step};
use super::*;
use crate::{
    ModelCollider3dComponent, ModelCollisionGeometry3d, ModelConvexCollider3dComponent,
    ModelConvexGeometry3d, PhysicsMaterial3dComponent, PhysicsMaterialSources,
};
use serde_json::{Value, json};
use sindri_core::{AssetId, EntityData, ProfileDocument, SceneComponent, Transform3D};
use sindri_physics::{PhysicsEventKind, PhysicsMaterial};
use std::sync::Arc;

fn sources(friction: f32, restitution: f32) -> PhysicsMaterialSources {
    let profile = ProfileDocument::from_json(&json!({"format_version":1,"type":"physics_material","values":{"friction":friction,"restitution":restitution}}).to_string()).unwrap();
    PhysicsMaterialSources::from_profiles([("shared.profile", &profile)]).unwrap()
}
fn author(world: &mut World, entity: EntityId, value: Value) {
    world
        .get_mut(entity)
        .unwrap()
        .components
        .insert(PhysicsMaterial3dComponent::TYPE_NAME.into(), value);
}
fn coefficients(
    physics: &ScenePhysics3d,
    entity: EntityId,
    count: usize,
    friction: f32,
    restitution: f32,
) {
    assert_eq!(
        physics.world().materials(entity).unwrap(),
        vec![
            PhysicsMaterial {
                friction,
                restitution
            };
            count
        ]
    );
}
fn hull() -> Arc<ModelConvexGeometry3d> {
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

#[test]
fn mixed_hull_reload_overrides_removal_and_literal_edits_preserve_motion() {
    let mut world = World::default();
    let actor = spawn(
        &mut world,
        [0.0, 3.0, 0.0],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.25),
    );
    world.get_mut(actor).unwrap().components.insert(
        ModelConvexCollider3dComponent::TYPE_NAME.into(),
        json!({"asset":"cube.glb"}),
    );
    author(
        &mut world,
        actor,
        json!({"profile":"shared.profile","override_restitution":true,"restitution":0.2}),
    );
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    physics.bind_model_convex_collision(AssetId::new("cube.glb").unwrap(), hull());
    physics.set_materials(sources(0.3, 0.8));
    step(&mut physics, &mut world);
    coefficients(&physics, actor, 2, 0.3, 0.2);
    let mass = physics.world().mass(actor).unwrap();
    physics
        .world_mut()
        .set_linear_velocity(actor, [2.0, 1.0, 0.0])
        .unwrap();
    physics
        .world_mut()
        .set_angular_velocity(actor, [0.0, 0.5, 0.0])
        .unwrap();
    physics.set_materials(sources(0.0, 0.9));
    step(&mut physics, &mut world);
    coefficients(&physics, actor, 2, 0.0, 0.2);
    near3(
        physics.world().linear_velocity(actor).unwrap(),
        [2.0, 1.0, 0.0],
    );
    near3(
        physics.world().angular_velocity(actor).unwrap(),
        [0.0, 0.5, 0.0],
    );
    assert!((physics.world().mass(actor).unwrap() - mass).abs() < 0.001);
    author(
        &mut world,
        actor,
        json!({"profile": "shared.profile", "override_restitution": false, "restitution": -1.0}),
    );
    step(&mut physics, &mut world);
    coefficients(&physics, actor, 2, 0.0, 0.9);
    world
        .get_mut(actor)
        .unwrap()
        .components
        .remove(PhysicsMaterial3dComponent::TYPE_NAME);
    step(&mut physics, &mut world);
    coefficients(&physics, actor, 2, 0.5, 0.0);
    // The literal hull coefficient follows the same nonstructural path.
    world
        .get_mut(actor)
        .unwrap()
        .components
        .get_mut(ModelConvexCollider3dComponent::TYPE_NAME)
        .unwrap()["friction"] = json!(0.1);
    step(&mut physics, &mut world);
    assert!((physics.world().materials(actor).unwrap()[1].friction - 0.1).abs() < 0.001);
    near3(
        physics.world().linear_velocity(actor).unwrap(),
        [2.0, 1.0, 0.0],
    );
    let mut replay = physics.clone();
    let mut snapshot = world.clone();
    step(&mut physics, &mut world);
    step(&mut replay, &mut snapshot);
    assert_eq!(
        world.world_transform(actor),
        snapshot.world_transform(actor)
    );
    // Shape edits still rebuild according to the existing authoring contract.
    world
        .get_mut(actor)
        .unwrap()
        .components
        .get_mut(Collider3dComponent::TYPE_NAME)
        .unwrap()["pieces"][0]["shape"]["radius"] = json!(0.3);
    step(&mut physics, &mut world);
    near3(physics.world().linear_velocity(actor).unwrap(), [0.0; 3]);
}

#[test]
fn bad_reference_or_override_leaves_whole_live_batch_and_events_unchanged() {
    let mut world = World::default();
    let first = spawn(
        &mut world,
        [0.0; 3],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.5),
    );
    let second = spawn(&mut world, [3.0, 0.0, 0.0], None, Collider3d::sphere(0.5));
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    physics.set_materials(sources(0.1, 0.7));
    step(&mut physics, &mut world);
    physics
        .world_mut()
        .set_linear_velocity(first, [2.0, 0.0, 0.0])
        .unwrap();
    let pose = physics.world().pose(first).unwrap();
    let events = physics.events().to_vec();
    author(&mut world, first, json!({"profile":"shared.profile"}));
    for invalid in [
        json!({"profile":"missing.profile","override_friction":true,"friction":0.1,"override_restitution":true,"restitution":0.0}),
        json!({"profile":"shared.profile","override_restitution":true,"restitution":1.1}),
    ] {
        author(&mut world, second, invalid);
        assert!(
            physics
                .step(&mut world, &components(), Duration::from_millis(10))
                .is_err()
        );
        coefficients(&physics, first, 1, 0.5, 0.0);
        assert_eq!(physics.world().pose(first).unwrap(), pose);
        assert_eq!(physics.events(), events);
        near3(
            physics.world().linear_velocity(first).unwrap(),
            [2.0, 0.0, 0.0],
        );
    }
    author(&mut world, second, json!({"profile":"shared.profile"}));
    step(&mut physics, &mut world);
    coefficients(&physics, first, 1, 0.1, 0.7);
    world.get_mut(second).unwrap().disabled = true;
    author(&mut world, second, json!({"profile":"inactive.profile"}));
    step(&mut physics, &mut world);
    let refs = crate::referenced_physics_materials(&world, &components()).unwrap();
    assert!(refs.contains(&"inactive.profile".into()));
}

#[test]
fn static_model_profile_changes_existing_contact_friction_without_restart() {
    let mut world = World::default();
    let floor = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [(
            ModelCollider3dComponent::TYPE_NAME.into(),
            json!({"asset":"floor.glb"}),
        )]
        .into(),
        ..EntityData::default()
    });
    let actor = spawn(
        &mut world,
        [0.0, 1.0, 0.0],
        Some(RigidBodyKind::Dynamic),
        Collider3d::cuboid([0.5; 3]),
    );
    world
        .get_mut(actor)
        .unwrap()
        .components
        .get_mut(RigidBody3dComponent::TYPE_NAME)
        .unwrap()["lock_rotation"] = json!(true);
    author(&mut world, floor, json!({"profile":"shared.profile"}));
    let mut physics = ScenePhysics3d::new([0.0, -9.81, 0.0]).unwrap();
    physics.model_collision_bindings_mut().bind(
        AssetId::new("floor.glb").unwrap(),
        Arc::new(
            ModelCollisionGeometry3d::new(
                vec![
                    [-10.0, 0.0, -10.0],
                    [-10.0, 0.0, 10.0],
                    [10.0, 0.0, 10.0],
                    [10.0, 0.0, -10.0],
                ],
                vec![[0, 1, 2], [0, 2, 3]],
            )
            .unwrap(),
        ),
    );
    physics.set_materials(sources(1.0, 0.0));
    for _ in 0..300 {
        step(&mut physics, &mut world);
    }
    physics.set_materials(sources(0.0, 0.0));
    physics
        .world_mut()
        .apply_impulse(actor, [2.0, 0.0, 0.0])
        .unwrap();
    for _ in 0..100 {
        step(&mut physics, &mut world);
        assert!(
            !physics
                .events()
                .iter()
                .any(|event| event.kind == PhysicsEventKind::CollisionStarted)
        );
    }
    assert!(world.world_transform(actor).unwrap().position[0] > 1.5);
    coefficients(&physics, floor, 1, 0.0, 0.0);
}

#[test]
fn reload_preserves_character_support_and_pending_platform_target() {
    let mut world = World::default();
    let platform = spawn(
        &mut world,
        [0.0, -0.5, 0.0],
        Some(RigidBodyKind::KinematicPosition),
        Collider3d::cuboid([5.0, 0.5, 5.0]),
    );
    let actor = spawn(&mut world, [0.0, 0.51, 0.0], None, Collider3d::sphere(0.5));
    world
        .get_mut(actor)
        .unwrap()
        .components
        .insert(crate::Character3dComponent::TYPE_NAME.into(), json!({}));
    for entity in [platform, actor] {
        author(&mut world, entity, json!({"profile":"shared.profile"}));
    }
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    physics.set_materials(sources(0.5, 0.0));
    step(&mut physics, &mut world);
    assert!(physics.character_motion(actor).unwrap().grounded);
    physics
        .world_mut()
        .set_kinematic_target(
            platform,
            PhysicsPose3d {
                position: [1.0, -0.5, 0.0],
                ..PhysicsPose3d::default()
            },
        )
        .unwrap();
    physics.set_materials(sources(0.0, 0.1));
    step(&mut physics, &mut world);
    coefficients(&physics, actor, 1, 0.0, 0.1);
    coefficients(&physics, platform, 1, 0.0, 0.1);
    near3(
        world.world_transform(actor).unwrap().position,
        [1.0, 0.51, 0.0],
    );
    let motion = physics.character_motion(actor).unwrap();
    assert!(motion.grounded);
    near3(
        motion.platform.as_ref().unwrap().motion.translation,
        [1.0, 0.0, 0.0],
    );
    step(&mut physics, &mut world);
    near3(
        physics.character_motion(actor).unwrap().translation,
        [0.0; 3],
    );
}
