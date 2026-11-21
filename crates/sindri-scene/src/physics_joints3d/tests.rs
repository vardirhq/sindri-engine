//! Authored 3D ropes through the scene driver.
use std::time::Duration;

use serde_json::json;
use sindri_core::{EntityData, EntityId, SceneComponent, SceneEntityId, Transform3D, World};
use sindri_physics::{Collider3d, RigidBody3d, RigidBodyKind};

use super::DistanceJoint3dComponent;
use crate::{
    Collider3dComponent, PhysicsSyncError, RigidBody3dComponent, SceneExtractor, ScenePhysics3d,
};

const STEP: Duration = Duration::from_millis(16);

fn body(world: &mut World, id: &str, x: f32, kind: RigidBodyKind, speed: f32) -> EntityId {
    world.spawn(EntityData {
        source_id: SceneEntityId::new(id).ok(),
        transform_3d: Some(Transform3D {
            position: [x, 0.0, 0.0],
            ..Transform3D::default()
        }),
        components: [
            (
                Collider3dComponent::TYPE_NAME.into(),
                json!({"pieces": [Collider3d::sphere(0.1)]}),
            ),
            (
                RigidBody3dComponent::TYPE_NAME.into(),
                serde_json::to_value(RigidBody3d {
                    kind,
                    gravity_scale: 0.0,
                    linear_velocity: [speed, 0.0, 0.0],
                    ..RigidBody3d::default()
                })
                .unwrap(),
            ),
        ]
        .into(),
        ..EntityData::default()
    })
}

fn rope(world: &mut World, length: f32) -> EntityId {
    world.spawn(EntityData {
        source_id: SceneEntityId::new("rope").ok(),
        components: [(
            DistanceJoint3dComponent::TYPE_NAME.into(),
            json!({"first": "post", "second": "ball", "max_distance": length}),
        )]
        .into(),
        ..EntityData::default()
    })
}

fn set(world: &mut World, owner: EntityId, field: &str, value: serde_json::Value) {
    world
        .get_mut(owner)
        .unwrap()
        .components
        .get_mut(DistanceJoint3dComponent::TYPE_NAME)
        .unwrap()[field] = value;
}

fn x(world: &World, entity: EntityId) -> f32 {
    world.world_transform(entity).unwrap().position[0]
}

fn run(physics: &mut ScenePhysics3d, world: &mut World, steps: usize) {
    let registry = SceneExtractor::new().unwrap().components().clone();
    for _ in 0..steps {
        physics.step(world, &registry, STEP).unwrap();
    }
}

#[test]
fn an_authored_rope_holds_a_body_and_retunes_and_suspends_in_place() {
    let mut world = World::default();
    body(&mut world, "post", 0.0, RigidBodyKind::Static, 0.0);
    let ball = body(&mut world, "ball", 1.0, RigidBodyKind::Dynamic, 2.0);
    let owner = rope(&mut world, 2.0);
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    run(&mut physics, &mut world, 120);
    assert!((x(&world, ball) - 2.0).abs() < 0.05, "{}", x(&world, ball));
    assert!(
        (physics.world().distance_joint(owner).unwrap().max_distance - 2.0).abs() < f32::EPSILON
    );

    // Shortening the authored length reels the ball in.
    for tenths in (10..20_u8).rev() {
        set(
            &mut world,
            owner,
            "max_distance",
            json!(f32::from(tenths) / 10.0),
        );
        run(&mut physics, &mut world, 6);
    }
    run(&mut physics, &mut world, 30);
    assert!(x(&world, ball) < 1.06, "{}", x(&world, ball));

    // Suspended, the ball leaves; enabled again, the rope catches it.
    physics
        .world_mut()
        .set_linear_velocity(ball, [2.0, 0.0, 0.0])
        .unwrap();
    set(&mut world, owner, "enabled", json!(false));
    run(&mut physics, &mut world, 30);
    assert!(physics.world().distance_joint(owner).is_none());
    assert!(x(&world, ball) > 1.5, "{}", x(&world, ball));
    set(&mut world, owner, "enabled", json!(true));
    run(&mut physics, &mut world, 60);
    assert!(physics.world().distance_joint(owner).is_some());
    assert!(x(&world, ball) < 2.6, "caught again: {}", x(&world, ball));
}

#[test]
fn missing_endpoints_suspend_and_invalid_lengths_fail_before_anything_moves() {
    let mut world = World::default();
    body(&mut world, "post", 0.0, RigidBodyKind::Static, 0.0);
    let owner = rope(&mut world, 1.0);
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    run(&mut physics, &mut world, 2);
    assert!(
        physics.world().distance_joint(owner).is_none(),
        "no ball yet"
    );
    let ball = body(&mut world, "ball", 0.5, RigidBodyKind::Dynamic, 3.0);
    run(&mut physics, &mut world, 60);
    assert!(physics.world().distance_joint(owner).is_some());
    assert!(x(&world, ball) < 1.05, "{}", x(&world, ball));

    set(&mut world, owner, "max_distance", json!(-1.0));
    let before = x(&world, ball);
    let registry = SceneExtractor::new().unwrap().components().clone();
    assert!(matches!(
        physics.step(&mut world, &registry, STEP),
        Err(PhysicsSyncError::Physics(_))
    ));
    assert!((x(&world, ball) - before).abs() < f32::EPSILON);

    // Removing the component releases the constraint.
    world.get_mut(owner).unwrap().components.clear();
    run(&mut physics, &mut world, 1);
    assert!(physics.world().distance_joint(owner).is_none());
}
