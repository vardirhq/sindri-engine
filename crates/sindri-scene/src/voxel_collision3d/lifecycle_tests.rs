//! Resident ownership and actual edited voxel collision lifecycle.

use super::*;
use sindri_physics::PhysicsEventKind;
use std::time::Duration;

#[test]
fn departure_inactivity_and_reentry_release_real_owner_geometry() {
    let mut world = World::default();
    let owner = world.spawn(EntityData::default());
    let mut physics = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let mut cache = SceneVoxelCollision3d::default();
    let voxels = section();
    let sections = [VoxelCollisionSection3d {
        coord: SectionCoord::new(-1, 0, -1),
        revision: 1,
        voxels: &voxels,
    }];
    cache
        .synchronize(&world, &mut physics, &[input(owner, &sections, &solid, 1)])
        .unwrap();
    assert_eq!(hit(&physics, -15.5, -15.5).unwrap().entity, owner);
    let report = cache
        .synchronize(&world, &mut physics, &[input(owner, &[], &solid, 1)])
        .unwrap();
    assert_eq!(report.removed, 1);
    assert!(hit(&physics, -15.5, -15.5).is_none());
    cache
        .synchronize(&world, &mut physics, &[input(owner, &sections, &solid, 1)])
        .unwrap();
    world.get_mut(owner).unwrap().disabled = true;
    cache
        .synchronize(&world, &mut physics, &[input(owner, &sections, &solid, 1)])
        .unwrap();
    assert!(!physics.contains(owner));
    world.get_mut(owner).unwrap().disabled = false;
    cache
        .synchronize(&world, &mut physics, &[input(owner, &sections, &solid, 1)])
        .unwrap();
    cache.synchronize(&world, &mut physics, &[]).unwrap();
    assert!(!physics.contains(owner));
}

#[test]
fn solver_reset_requires_a_fresh_cache_and_stale_input_owners_are_rejected() {
    let mut world = World::default();
    let owner = world.spawn(EntityData::default());
    let mut physics = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let mut cache = SceneVoxelCollision3d::default();
    let voxels = section();
    let sections = [VoxelCollisionSection3d {
        coord: SectionCoord::default(),
        revision: 1,
        voxels: &voxels,
    }];
    cache
        .synchronize(&world, &mut physics, &[input(owner, &sections, &solid, 1)])
        .unwrap();
    physics.remove(owner);
    assert!(matches!(
        cache.synchronize(&world, &mut physics, &[]),
        Err(VoxelCollisionError3d::Ownership(_))
    ));
    assert_eq!(cache.owners.len(), 1);
    let mut cache = SceneVoxelCollision3d::default();
    cache
        .synchronize(&world, &mut physics, &[input(owner, &sections, &solid, 1)])
        .unwrap();
    world.despawn_recursive(owner).unwrap();
    assert!(matches!(
        cache.synchronize(&world, &mut physics, &[input(owner, &sections, &solid, 1)]),
        Err(VoxelCollisionError3d::MissingOwner(_))
    ));
    cache.synchronize(&world, &mut physics, &[]).unwrap();
    assert!(!physics.contains(owner));
}

#[test]
fn edited_occupied_section_supports_real_contact_then_the_opened_hole_allows_falling() {
    let mut world = World::default();
    let owner = world.spawn(EntityData::default());
    let actor = world.spawn(EntityData::default());
    let mut physics = PhysicsWorld3d::new([0.0, -9.81, 0.0]).unwrap();
    let mut cache = SceneVoxelCollision3d::default();
    let mut voxels = VoxelSection::default();
    for z in 0..16 {
        for x in 0..16 {
            voxels.set(LocalVoxelCoord::new(x, 0, z), VoxelId::new(1));
        }
    }
    let sections = [VoxelCollisionSection3d {
        coord: SectionCoord::default(),
        revision: 1,
        voxels: &voxels,
    }];
    let report = cache
        .synchronize(&world, &mut physics, &[input(owner, &sections, &solid, 1)])
        .unwrap();
    assert_eq!(report.pieces, 1);
    physics
        .insert_body(
            actor,
            RigidBody3d {
                position: [0.5, 3.0, 0.5],
                lock_rotation: true,
                ..RigidBody3d::default()
            },
            &[Collider3d::cuboid([0.2; 3])],
        )
        .unwrap();
    let mut touched = false;
    for _ in 0..150 {
        touched |= physics
            .step(Duration::from_millis(10))
            .unwrap()
            .iter()
            .any(|event| {
                event.kind == PhysicsEventKind::CollisionStarted
                    && [event.first, event.second].contains(&owner)
            });
    }
    assert!(touched);
    assert!((physics.pose(actor).unwrap().position[1] - 1.2).abs() < 0.04);
    voxels.set(LocalVoxelCoord::new(0, 0, 0), VoxelId::AIR);
    let sections = [VoxelCollisionSection3d {
        coord: SectionCoord::default(),
        revision: 2,
        voxels: &voxels,
    }];
    cache
        .synchronize(&world, &mut physics, &[input(owner, &sections, &solid, 1)])
        .unwrap();
    assert!(
        physics
            .raycast(
                [0.5, 0.9, 0.5],
                [0.0, -1.0, 0.0],
                2.0,
                RaycastFilter3d::default()
            )
            .unwrap()
            .is_none()
    );
    for _ in 0..70 {
        physics.step(Duration::from_millis(10)).unwrap();
    }
    assert!(physics.pose(actor).unwrap().position[1] < 0.0);
}
