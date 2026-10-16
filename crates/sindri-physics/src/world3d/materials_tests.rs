use super::*;
use crate::{
    Collider3d, ConvexHull3d, ConvexHullCollider3d, PhysicsEventKind, PhysicsPose3d,
    PhysicsWorld3d, RigidBody3d, RigidBodyKind, TriangleMesh3d, TriangleMeshCollider3d,
};
use std::time::Duration;

const STEP: Duration = Duration::from_millis(16);

fn id(value: u64) -> EntityId {
    EntityId::from_bits(value << 32)
}

fn material(friction: f32, restitution: f32) -> PhysicsMaterial {
    PhysicsMaterial {
        friction,
        restitution,
    }
}

fn hull() -> ConvexHullCollider3d {
    ConvexHullCollider3d::new(
        ConvexHull3d::new(&[
            [-0.5, -0.5, -0.5],
            [-0.5, -0.5, 0.5],
            [-0.5, 0.5, -0.5],
            [-0.5, 0.5, 0.5],
            [0.5, -0.5, -0.5],
            [0.5, -0.5, 0.5],
            [0.5, 0.5, -0.5],
            [0.5, 0.5, 0.5],
        ])
        .unwrap(),
    )
}

fn floor() -> TriangleMeshCollider3d {
    TriangleMeshCollider3d::new(
        TriangleMesh3d::new(
            &[
                [-10.0, 0.0, -10.0],
                [-10.0, 0.0, 10.0],
                [10.0, 0.0, 10.0],
                [10.0, 0.0, -10.0],
            ],
            &[[0, 1, 2], [0, 2, 3]],
        )
        .unwrap(),
    )
}

#[test]
fn invalid_compound_edits_are_atomic_and_valid_edits_keep_state() {
    let entity = id(1);
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_hull_body(
            entity,
            RigidBody3d::default(),
            &[Collider3d::sphere(0.5)],
            &[hull()],
        )
        .unwrap();
    world.set_linear_velocity(entity, [2.0, 3.0, 4.0]).unwrap();
    world.set_angular_velocity(entity, [0.1, 0.2, 0.3]).unwrap();
    let record = world.record(entity).unwrap().clone();
    world.backend.bodies[record.body].add_force(super::super::r3::Vector::X, true);
    let original = world.materials(entity).unwrap();
    let pose = world.pose(entity).unwrap();
    let mass = world.mass(entity).unwrap();
    let force = world.backend.bodies[record.body].user_force();
    for invalid in [
        material(f32::NAN, 0.0),
        material(f32::INFINITY, 0.0),
        material(-0.1, 0.0),
        material(0.5, -0.1),
        material(0.5, 1.1),
        material(0.5, f32::INFINITY),
    ] {
        assert!(
            world
                .set_materials(entity, &[material(0.1, 0.9), invalid])
                .is_err()
        );
        assert_eq!(world.materials(entity).unwrap(), original);
    }
    assert!(matches!(
        world.set_materials(entity, &[material(0.0, 0.0)]),
        Err(PhysicsError::MaterialPieceCount {
            expected: 2,
            actual: 1
        })
    ));
    assert!(matches!(
        world.materials(id(2)),
        Err(PhysicsError::MissingEntity(_))
    ));
    assert!(matches!(
        world.set_materials(id(2), &[]),
        Err(PhysicsError::MissingEntity(_))
    ));
    let replacement = [material(0.1, 0.9), material(0.2, 0.8)];
    world.set_materials(entity, &replacement).unwrap();
    assert_eq!(world.materials(entity).unwrap(), replacement);
    assert_eq!(world.record(entity).unwrap().body, record.body);
    assert_eq!(world.record(entity).unwrap().colliders, record.colliders);
    assert_eq!(world.pose(entity).unwrap(), pose);
    assert!((world.mass(entity).unwrap() - mass).abs() < f32::EPSILON);
    assert_eq!(
        world.linear_velocity(entity).unwrap().map(f32::to_bits),
        [2.0_f32, 3.0, 4.0].map(f32::to_bits)
    );
    assert_eq!(
        world.angular_velocity(entity).unwrap().map(f32::to_bits),
        [0.1_f32, 0.2, 0.3].map(f32::to_bits)
    );
    assert_eq!(world.backend.bodies[record.body].user_force(), force);
    let mut replay = world.clone();
    assert_eq!(world.step(STEP).unwrap(), replay.step(STEP).unwrap());
    assert_eq!(world.pose(entity).unwrap(), replay.pose(entity).unwrap());
    assert_eq!(
        world.materials(entity).unwrap(),
        replay.materials(entity).unwrap()
    );
}

#[test]
fn static_mesh_and_groups_use_stable_piece_order_without_replacement() {
    let entity = id(1);
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_static_mesh(entity, PhysicsPose3d::default(), &[floor()])
        .unwrap();
    world
        .replace_static_group(
            entity,
            20,
            PhysicsPose3d::default(),
            &[Collider3d::sphere(0.2)],
        )
        .unwrap();
    world
        .replace_static_mesh_group(entity, 10, PhysicsPose3d::default(), &[floor()])
        .unwrap();
    let record = world.record(entity).unwrap().clone();
    let replacement = [material(0.1, 0.0), material(0.2, 0.1), material(0.3, 0.2)];
    world.set_materials(entity, &replacement).unwrap();
    assert_eq!(world.materials(entity).unwrap(), replacement);
    assert_eq!(world.record(entity).unwrap().body, record.body);
    assert_eq!(world.record(entity).unwrap().colliders, record.colliders);
    assert_eq!(world.record(entity).unwrap().groups, record.groups);
    world.remove_static_group(entity, 10).unwrap();
    assert_eq!(
        world.materials(entity).unwrap(),
        [replacement[0], replacement[2]]
    );
    world.remove_static_group(entity, 20).unwrap();
    assert_eq!(world.materials(entity).unwrap(), [replacement[0]]);
    world
        .replace_static_group(
            id(2),
            0,
            PhysicsPose3d::default(),
            &[Collider3d::sphere(0.2)],
        )
        .unwrap();
    world.remove_static_group(id(2), 0).unwrap();
    world.set_materials(id(2), &[]).unwrap();
    assert!(world.materials(id(2)).unwrap().is_empty());
}

#[test]
fn identical_materials_leave_sleeping_body_asleep() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_body(id(1), RigidBody3d::default(), &[Collider3d::sphere(0.5)])
        .unwrap();
    let handle = world.record(id(1)).unwrap().body;
    world.backend.bodies[handle].sleep();
    world
        .set_materials(id(1), &world.materials(id(1)).unwrap())
        .unwrap();
    assert!(world.backend.bodies[handle].is_sleeping());
    assert!(!world.materials_changed);
    world.set_materials(id(1), &[material(0.0, 0.8)]).unwrap();
    assert!(!world.backend.bodies[handle].is_sleeping());
}

#[test]
fn edits_preserve_position_kinematic_target() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_body(
            id(1),
            RigidBody3d {
                kind: RigidBodyKind::KinematicPosition,
                ..RigidBody3d::default()
            },
            &[Collider3d::sphere(0.5)],
        )
        .unwrap();
    world
        .set_kinematic_target(
            id(1),
            PhysicsPose3d {
                position: [2.0, 3.0, 4.0],
                ..PhysicsPose3d::default()
            },
        )
        .unwrap();
    world.set_materials(id(1), &[material(0.0, 0.8)]).unwrap();
    world.step(STEP).unwrap();
    assert_eq!(
        world.pose(id(1)).unwrap().position.map(f32::to_bits),
        [2.0_f32, 3.0, 4.0].map(f32::to_bits)
    );
}

#[test]
fn existing_contacts_use_new_friction_without_new_start_events() {
    let mut world = PhysicsWorld3d::new([0.0, -9.81, 0.0]).unwrap();
    world
        .insert_static_mesh(id(1), PhysicsPose3d::default(), &[floor()])
        .unwrap();
    world
        .insert_hull_body(
            id(2),
            RigidBody3d {
                position: [0.0, 1.0, 0.0],
                lock_rotation: true,
                ..RigidBody3d::default()
            },
            &[],
            &[hull()],
        )
        .unwrap();
    world.set_materials(id(1), &[material(1.0, 0.0)]).unwrap();
    world.set_materials(id(2), &[material(1.0, 0.0)]).unwrap();
    for _ in 0..240 {
        world.step(STEP).unwrap();
    }
    let mut unchanged = world.clone();
    world.set_materials(id(1), &[material(0.0, 0.0)]).unwrap();
    for trial in [&mut world, &mut unchanged] {
        trial.apply_impulse(id(2), [2.0, 0.0, 0.0]).unwrap();
        for _ in 0..60 {
            let events = trial.step(STEP).unwrap();
            assert!(
                !events
                    .iter()
                    .any(|event| event.kind == PhysicsEventKind::CollisionStarted)
            );
        }
    }
    let slippery = world.pose(id(2)).unwrap().position[0];
    let rough = unchanged.pose(id(2)).unwrap().position[0];
    assert!(slippery > 1.5, "slippery floor: {slippery}");
    assert!(rough < 0.8, "rough floor: {rough}");
}

#[test]
fn changed_static_material_wakes_sleeping_contact_neighbor() {
    let mut world = PhysicsWorld3d::new([0.0, -9.81, 0.0]).unwrap();
    world
        .insert_static_mesh(id(1), PhysicsPose3d::default(), &[floor()])
        .unwrap();
    world
        .insert_body(
            id(2),
            RigidBody3d {
                position: [0.0, 1.0, 0.0],
                lock_rotation: true,
                ..RigidBody3d::default()
            },
            &[Collider3d::cuboid([0.5; 3])],
        )
        .unwrap();
    for _ in 0..240 {
        world.step(STEP).unwrap();
    }
    let handle = world.record(id(2)).unwrap().body;
    assert!(world.backend.bodies[handle].is_sleeping());
    world.backend.bodies[handle].add_force(super::super::r3::Vector::X * 2.0, false);
    let mut unchanged = world.clone();
    world.set_materials(id(1), &[material(0.0, 0.0)]).unwrap();
    assert!(!world.backend.bodies[handle].is_sleeping());
    let recycle = world
        .backend
        .integration_parameters
        .normalized_contact_recycle_distance;
    assert!(world.step(Duration::ZERO).is_err());
    assert!(world.materials_changed);
    let mut replay = world.clone();
    for _ in 0..60 {
        assert_eq!(world.step(STEP).unwrap(), replay.step(STEP).unwrap());
        assert_eq!(world.pose(id(2)).unwrap(), replay.pose(id(2)).unwrap());
        unchanged.step(STEP).unwrap();
    }
    assert!(!world.materials_changed);
    assert_eq!(
        world
            .backend
            .integration_parameters
            .normalized_contact_recycle_distance
            .to_bits(),
        recycle.to_bits()
    );
    let position = world.pose(id(2)).unwrap().position;
    let velocity = world.linear_velocity(id(2)).unwrap();
    assert!(
        position[0] > 0.75,
        "position {position:?}, velocity {velocity:?}"
    );
    assert!(unchanged.pose(id(2)).unwrap().position[0].abs() < 0.01);
}

#[test]
fn edited_restitution_changes_rebound_and_snapshot_replays_it() {
    let mut world = PhysicsWorld3d::new([0.0, -9.81, 0.0]).unwrap();
    world
        .insert_static_mesh(id(1), PhysicsPose3d::default(), &[floor()])
        .unwrap();
    world
        .insert_body(
            id(2),
            RigidBody3d {
                position: [0.0, 3.0, 0.0],
                ..RigidBody3d::default()
            },
            &[Collider3d::sphere(0.5)],
        )
        .unwrap();
    for _ in 0..20 {
        world.step(STEP).unwrap();
    }
    let mut unchanged = world.clone();
    for entity in [id(1), id(2)] {
        world.set_materials(entity, &[material(0.0, 1.0)]).unwrap();
    }
    let mut replay = world.clone();
    let mut landed = false;
    let mut peak = 0.0_f32;
    for _ in 0..90 {
        let events = world.step(STEP).unwrap();
        assert_eq!(events, replay.step(STEP).unwrap());
        assert_eq!(world.pose(id(2)).unwrap(), replay.pose(id(2)).unwrap());
        landed |= events
            .iter()
            .any(|event| event.kind == PhysicsEventKind::CollisionStarted);
        if landed {
            peak = peak.max(world.pose(id(2)).unwrap().position[1]);
        }
        unchanged.step(STEP).unwrap();
    }
    assert!(landed && peak > 2.0, "rebound peak: {peak}");
    assert!(unchanged.pose(id(2)).unwrap().position[1] < 0.6);
}
