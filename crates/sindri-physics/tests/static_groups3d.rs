//! Streaming/editable static pieces share one real owner without body rebuilds.

use sindri_core::EntityId;
use sindri_physics::{
    BodyControl3d, Collider3d, CollisionLayers, PhysicsEventKind, PhysicsPose3d, PhysicsWorld3d,
    RaycastFilter3d, RigidBody3d, RigidBodyKind,
};
use std::time::Duration;

fn id(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}
fn at(x: f32) -> Collider3d {
    Collider3d {
        offset: [x, 0.0, 0.0],
        ..Collider3d::cuboid([0.5; 3])
    }
}
fn hit(world: &PhysicsWorld3d, x: f32) -> Option<EntityId> {
    world
        .raycast(
            [x, 2.0, 0.0],
            [0.0, -1.0, 0.0],
            4.0,
            RaycastFilter3d::default(),
        )
        .unwrap()
        .map(|hit| hit.entity)
}

#[test]
fn replacing_and_removing_one_group_preserves_other_groups_and_original_pieces() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let owner = id(1);
    let pose = PhysicsPose3d::default();
    world
        .insert_static_collider(owner, pose, &[at(-3.0)])
        .unwrap();
    world
        .replace_static_group(owner, 10, pose, &[at(0.0)])
        .unwrap();
    world
        .replace_static_group(owner, 20, pose, &[at(3.0)])
        .unwrap();
    for x in [-3.0, 0.0, 3.0] {
        assert_eq!(hit(&world, x), Some(owner));
    }
    world
        .replace_static_group(owner, 10, pose, &[at(1.5)])
        .unwrap();
    assert_eq!(hit(&world, 0.0), None);
    for x in [-3.0, 1.5, 3.0] {
        assert_eq!(hit(&world, x), Some(owner));
    }
    assert!(world.remove_static_group(owner, 20).unwrap());
    assert!(!world.remove_static_group(owner, 20).unwrap());
    assert_eq!(hit(&world, 3.0), None);
    assert_eq!(hit(&world, -3.0), Some(owner));
    world
        .replace_static_group(
            owner,
            10,
            PhysicsPose3d {
                position: [1.0, 0.0, 0.0],
                ..pose
            },
            &[],
        )
        .unwrap();
    assert_eq!(hit(&world, -3.0), None);
    assert_eq!(hit(&world, -2.0), Some(owner));
}

#[test]
fn invalid_replacement_and_nonstatic_owners_fail_without_losing_geometry() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let owner = id(1);
    let pose = PhysicsPose3d::default();
    world
        .replace_static_group(owner, 0, pose, &[at(0.0)])
        .unwrap();
    assert!(
        world
            .replace_static_group(owner, 0, pose, &[at(3.0), Collider3d::sphere(-1.0)])
            .is_err()
    );
    assert!(
        world
            .replace_static_group(
                owner,
                0,
                PhysicsPose3d {
                    rotation: [0.0; 4],
                    ..pose
                },
                &[]
            )
            .is_err()
    );
    assert_eq!(hit(&world, 0.0), Some(owner));
    assert_eq!(hit(&world, 3.0), None);
    for (index, kind) in [
        RigidBodyKind::Dynamic,
        RigidBodyKind::KinematicVelocity,
        RigidBodyKind::KinematicPosition,
    ]
    .into_iter()
    .enumerate()
    {
        let entity = id(u32::try_from(index).unwrap() + 2);
        world
            .insert_body(
                entity,
                RigidBody3d {
                    kind,
                    position: [8.0, 0.0, 0.0],
                    ..RigidBody3d::default()
                },
                &[at(0.0)],
            )
            .unwrap();
        assert!(
            world
                .replace_static_group(entity, 0, pose, &[at(3.0)])
                .is_err()
        );
        assert!(world.remove_static_group(entity, 0).is_err());
        assert_eq!(hit(&world, 8.0), Some(id(2)));
    }
    let pending = id(10);
    world
        .remember_control(
            pending,
            RigidBodyKind::Dynamic,
            BodyControl3d::LinearVelocity([1.0; 3]),
        )
        .unwrap();
    assert!(
        world
            .replace_static_group(pending, 0, pose, &[at(0.0)])
            .is_err()
    );
    assert!(!world.contains(pending));
    assert_eq!(world.pending_linear_velocity(pending), Some([1.0; 3]));
}

#[test]
fn empty_groups_and_full_removal_do_not_leak_into_reused_owners() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let owner = id(1);
    let pose = PhysicsPose3d::default();
    world.replace_static_group(owner, 0, pose, &[]).unwrap();
    assert!(!world.contains(owner));
    world
        .replace_static_group(owner, 0, pose, &[at(0.0)])
        .unwrap();
    world.replace_static_group(owner, 0, pose, &[]).unwrap();
    assert!(world.contains(owner));
    assert_eq!(hit(&world, 0.0), None);
    world
        .replace_static_group(owner, 1, pose, &[at(3.0)])
        .unwrap();
    assert_eq!(hit(&world, 3.0), Some(owner));
    assert!(world.remove(owner));
    world
        .replace_static_group(owner, 0, pose, &[at(-3.0)])
        .unwrap();
    assert_eq!(hit(&world, 3.0), None);
    assert_eq!(hit(&world, -3.0), Some(owner));
}

#[test]
fn group_queries_keep_local_pose_sensor_and_membership_filters() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let owner = id(1);
    let sensor = Collider3d {
        sensor: true,
        layers: CollisionLayers::new(4, u32::MAX),
        ..at(2.0)
    };
    let pose = PhysicsPose3d {
        position: [1.0, 0.0, 0.0],
        ..PhysicsPose3d::default()
    };
    world.validate_static_group(owner, pose, &[sensor]).unwrap();
    assert!(!world.contains(owner));
    world
        .replace_static_group(owner, 0, pose, &[sensor])
        .unwrap();
    assert_eq!(hit(&world, 3.0), None);
    let cast = |mask, include_sensors| {
        world
            .raycast(
                [3.0, 2.0, 0.0],
                [0.0, -1.0, 0.0],
                4.0,
                RaycastFilter3d {
                    mask,
                    include_sensors,
                    ..RaycastFilter3d::default()
                },
            )
            .unwrap()
    };
    assert_eq!(cast(4, true).unwrap().entity, owner);
    assert!(cast(2, true).is_none());
    assert!(cast(4, false).is_none());
}

#[test]
fn group_solids_support_dynamic_bodies_and_events_name_the_real_owner() {
    let mut world = PhysicsWorld3d::new([0.0, -9.81, 0.0]).unwrap();
    let owner = id(1);
    let actor = id(2);
    world
        .replace_static_group(
            owner,
            0,
            PhysicsPose3d::default(),
            &[Collider3d::cuboid([3.0, 0.25, 3.0])],
        )
        .unwrap();
    world
        .insert_body(
            actor,
            RigidBody3d {
                position: [0.0, 2.0, 0.0],
                lock_rotation: true,
                ..RigidBody3d::default()
            },
            &[Collider3d::cuboid([0.5; 3])],
        )
        .unwrap();
    let mut touched = false;
    for _ in 0..150 {
        touched |= world
            .step(Duration::from_millis(10))
            .unwrap()
            .iter()
            .any(|event| {
                event.kind == PhysicsEventKind::CollisionStarted
                    && [event.first, event.second].contains(&owner)
            });
    }
    assert!(touched);
    assert!((world.pose(actor).unwrap().position[1] - 0.75).abs() < 0.04);
    world.remove_static_group(owner, 0).unwrap();
    for _ in 0..50 {
        world.step(Duration::from_millis(10)).unwrap();
    }
    assert!(world.pose(actor).unwrap().position[1] < 0.0);
}
