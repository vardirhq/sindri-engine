//! Parent-space write-back independent of entity allocation order.

use super::*;
use sindri_core::{EntityData, Transform3D};
use sindri_physics::Collider3d;

#[test]
fn moving_parent_and_child_write_world_poses_once_under_a_newer_rotated_parent() {
    let mut world = World::default();
    let child = super::tests::spawn(
        &mut world,
        [8.0, 0.0, 0.0],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.1),
    );
    let parent = super::tests::spawn(
        &mut world,
        [2.0, 0.0, 0.0],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.1),
    );
    let half = std::f32::consts::FRAC_1_SQRT_2;
    let transform = world
        .get_mut(parent)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap();
    transform.rotation = [0.0, half, 0.0, half];
    transform.scale = [2.0; 3];
    world.set_parent_keeping_place(child, Some(parent)).unwrap();
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    super::tests::step(&mut physics, &mut world);
    physics
        .world_mut()
        .set_linear_velocity(parent, [1.0, 0.0, 0.0])
        .unwrap();
    physics
        .world_mut()
        .set_linear_velocity(child, [0.0, 0.0, 1.0])
        .unwrap();
    for _ in 0..10 {
        super::tests::step(&mut physics, &mut world);
    }
    super::tests::near3(
        world.world_transform(parent).unwrap().position,
        [2.1, 0.0, 0.0],
    );
    super::tests::near3(
        world.world_transform(child).unwrap().position,
        [8.0, 0.0, 0.1],
    );
    for entity in [parent, child] {
        super::tests::near3(
            world.world_transform(entity).unwrap().position,
            physics.world().pose(entity).unwrap().position,
        );
    }
    super::tests::near3(world.world_transform(parent).unwrap().scale, [2.0; 3]);
    super::tests::near3(world.world_transform(child).unwrap().scale, [1.0; 3]);
    // Moving an unmodelled ancestor is an authored world-space teleport.
    let group = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        ..EntityData::default()
    });
    world.set_parent_keeping_place(parent, Some(group)).unwrap();
    world
        .get_mut(group)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position = [0.0, 3.0, 0.0];
    super::tests::step(&mut physics, &mut world);
    super::tests::near3(
        world.world_transform(child).unwrap().position,
        [8.0, 3.0, 0.11],
    );
    super::tests::near3(
        physics.world().linear_velocity(child).unwrap(),
        [0.0, 0.0, 1.0],
    );
}

#[test]
fn equivalent_quaternion_signs_do_not_count_as_external_motion() {
    let pose = PhysicsPose3d {
        rotation: [0.0, 0.0, 0.6, 0.8],
        ..PhysicsPose3d::default()
    };
    let opposite = PhysicsPose3d {
        rotation: [0.0, 0.0, -0.6, -0.8],
        ..pose
    };
    assert!(!moved(pose, opposite));
    assert!(moved(
        pose,
        PhysicsPose3d {
            position: [0.0, 0.0, 0.1],
            ..opposite
        }
    ));
}
