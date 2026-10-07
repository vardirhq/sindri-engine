//! Pre-solve replay and complete-batch rejection for spawn controls.

use super::*;
use serde_json::json;
use sindri_core::{EntityData, SceneComponent};
use sindri_physics::BodyControl3d;

fn step(physics: &mut ScenePhysics3d, world: &mut World) -> Result<(), PhysicsSyncError> {
    physics.step(world, &tests::components(), Duration::from_millis(10))
}

#[test]
fn first_materialization_replays_before_solving_and_preserves_authoring() {
    let mut world = World::default();
    let entity = tests::spawn(
        &mut world,
        [0.0; 3],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.5),
    );
    let authored = world.get(entity).unwrap().components.clone();
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    physics
        .world_mut()
        .remember_control(
            entity,
            RigidBodyKind::Dynamic,
            BodyControl3d::LinearVelocity([1.0, 2.0, 3.0]),
        )
        .unwrap();
    physics
        .world_mut()
        .remember_control(
            entity,
            RigidBodyKind::Dynamic,
            BodyControl3d::Impulse([1.0; 3]),
        )
        .unwrap();
    step(&mut physics, &mut world).unwrap();
    let kick = 1.0 / physics.world().mass(entity).unwrap();
    tests::near3(
        physics.world().linear_velocity(entity).unwrap(),
        [1.0 + kick, 2.0 + kick, 3.0 + kick],
    );
    assert!(physics.world().pose(entity).unwrap().position[0] > 0.01);
    assert_eq!(world.get(entity).unwrap().components, authored);
}

#[test]
fn queued_kind_conflict_rejects_the_whole_batch_and_retains_requests_for_retry() {
    let mut world = World::default();
    let old = tests::spawn(
        &mut world,
        [0.0; 3],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.5),
    );
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    step(&mut physics, &mut world).unwrap();
    let fresh = tests::spawn(
        &mut world,
        [5.0; 3],
        Some(RigidBodyKind::Static),
        Collider3d::sphere(0.5),
    );
    physics
        .world_mut()
        .remember_control(
            fresh,
            RigidBodyKind::Dynamic,
            BodyControl3d::Impulse([1.0; 3]),
        )
        .unwrap();
    world.get_mut(old).unwrap().disabled = true;
    world.spawn(EntityData {
        components: [(
            PhysicsWorld3dComponent::TYPE_NAME.into(),
            json!({"gravity": [1.0, 2.0, 3.0]}),
        )]
        .into(),
        ..EntityData::default()
    });
    assert!(step(&mut physics, &mut world).is_err());
    assert!(physics.world().contains(old));
    assert!(!physics.world().contains(fresh));
    tests::near3(physics.world().gravity(), [0.0; 3]);
    world
        .get_mut(fresh)
        .unwrap()
        .components
        .get_mut(RigidBody3dComponent::TYPE_NAME)
        .unwrap()["kind"] = json!("dynamic");
    step(&mut physics, &mut world).unwrap();
    assert!(!physics.world().contains(old));
    assert!(physics.world().linear_velocity(fresh).unwrap()[0] > 1.0);
}

#[test]
fn inactive_and_colliderless_entities_do_not_keep_unresolved_controls() {
    let mut world = World::default();
    let entity = tests::spawn(
        &mut world,
        [0.0; 3],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.5),
    );
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    for inactive in [true, false] {
        physics
            .world_mut()
            .remember_control(
                entity,
                RigidBodyKind::Dynamic,
                BodyControl3d::LinearVelocity([99.0; 3]),
            )
            .unwrap();
        world.get_mut(entity).unwrap().disabled = inactive;
        let collider = if inactive {
            None
        } else {
            world
                .get_mut(entity)
                .unwrap()
                .components
                .remove(Collider3dComponent::TYPE_NAME)
        };
        step(&mut physics, &mut world).unwrap();
        assert!(physics.world().linear_velocity(entity).is_err());
        assert!(physics.world().pending_linear_velocity(entity).is_none());
        world.get_mut(entity).unwrap().disabled = false;
        if let Some(collider) = collider {
            world
                .get_mut(entity)
                .unwrap()
                .components
                .insert(Collider3dComponent::TYPE_NAME.into(), collider);
        }
    }
    step(&mut physics, &mut world).unwrap();
    tests::near3(physics.world().linear_velocity(entity).unwrap(), [0.0; 3]);
}
