use super::*;
use sindri_core::EntityData;
use sindri_physics::{RaycastFilter3d, RigidBody3d};
use sindri_voxel::{LocalVoxelCoord, VoxelId, VoxelSection, VoxelShape};
use std::cell::Cell;

fn solid(voxel: VoxelId) -> Option<VoxelShape> {
    (!voxel.is_air()).then_some(VoxelShape::FULL)
}
fn section() -> VoxelSection {
    let mut section = VoxelSection::default();
    section.set(LocalVoxelCoord::new(0, 0, 0), VoxelId::new(1));
    section
}
fn input<'a>(
    owner: EntityId,
    sections: &'a [VoxelCollisionSection3d<'a>],
    policy: &'a dyn Fn(VoxelId) -> Option<VoxelShape>,
    revision: u64,
) -> VoxelCollisionWorld3d<'a> {
    VoxelCollisionWorld3d {
        owner,
        pose: PhysicsPose3d::default(),
        scale: [1.0; 3],
        policy_revision: revision,
        policy,
        settings: VoxelCollisionSettings3d::default(),
        sections,
    }
}
fn hit(physics: &PhysicsWorld3d, x: f32, z: f32) -> Option<sindri_physics::RayHit3d> {
    physics
        .raycast(
            [x, 8.0, z],
            [0.0, -1.0, 0.0],
            20.0,
            RaycastFilter3d::default(),
        )
        .unwrap()
}

#[test]
fn occupancy_and_policy_revisions_rebuild_only_changed_resident_sections() {
    let mut world = World::default();
    let owner = world.spawn(EntityData::default());
    let mut physics = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let mut cache = SceneVoxelCollision3d::default();
    let voxels = section();
    let sections = [
        VoxelCollisionSection3d {
            coord: SectionCoord::default(),
            revision: 1,
            voxels: &voxels,
        },
        VoxelCollisionSection3d {
            coord: SectionCoord::new(1, 0, 0),
            revision: 1,
            voxels: &voxels,
        },
    ];
    let classified = Cell::new(0);
    let policy = |voxel| {
        classified.set(classified.get() + 1);
        solid(voxel)
    };
    let report = cache
        .synchronize(&world, &mut physics, &[input(owner, &sections, &policy, 1)])
        .unwrap();
    assert_eq!((report.compiled, report.rebuilt, report.pieces), (2, 2, 2));
    let retained = cache.owners[&owner].sections[&sections[1].coord]
        .pieces
        .clone();
    let report = cache
        .synchronize(&world, &mut physics, &[input(owner, &sections, &policy, 1)])
        .unwrap();
    assert_eq!((report.compiled, report.rebuilt), (0, 0));
    assert_eq!(classified.get(), 2);
    let empty = VoxelSection::default();
    let edited = [
        VoxelCollisionSection3d {
            coord: sections[0].coord,
            revision: 2,
            voxels: &empty,
        },
        VoxelCollisionSection3d {
            coord: sections[1].coord,
            revision: 1,
            voxels: &voxels,
        },
    ];
    let report = cache
        .synchronize(&world, &mut physics, &[input(owner, &edited, &policy, 1)])
        .unwrap();
    assert_eq!((report.compiled, report.rebuilt), (1, 1));
    assert!(Arc::ptr_eq(
        &retained,
        &cache.owners[&owner].sections[&sections[1].coord].pieces
    ));
    assert!(hit(&physics, 0.5, 0.5).is_none());
    assert_eq!(hit(&physics, 16.5, 0.5).unwrap().entity, owner);
    let report = cache
        .synchronize(&world, &mut physics, &[input(owner, &edited, &|_| None, 2)])
        .unwrap();
    assert_eq!(report.compiled, 2);
    assert!(hit(&physics, 16.5, 0.5).is_none());
}

#[test]
fn transformed_partial_shapes_and_settings_reuse_compiled_geometry() {
    let mut world = World::default();
    let owner = world.spawn(EntityData::default());
    let mut physics = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let mut cache = SceneVoxelCollision3d::default();
    let mut voxels = VoxelSection::default();
    voxels.set(LocalVoxelCoord::new(1, 0, 0), VoxelId::new(1));
    let sections = [VoxelCollisionSection3d {
        coord: SectionCoord::default(),
        revision: 1,
        voxels: &voxels,
    }];
    let slab = |_| {
        Some(VoxelShape {
            min: [0; 3],
            max: [16, 8, 16],
        })
    };
    let mut snapshot = input(owner, &sections, &slab, 1);
    snapshot.scale = [2.0, 3.0, 4.0];
    snapshot.pose = PhysicsPose3d {
        position: [10.0, 0.0, 10.0],
        rotation: [
            0.0,
            std::f32::consts::FRAC_1_SQRT_2,
            0.0,
            std::f32::consts::FRAC_1_SQRT_2,
        ],
    };
    cache
        .synchronize(&world, &mut physics, &[snapshot])
        .unwrap();
    let touched = hit(&physics, 12.0, 7.0).unwrap();
    assert_eq!(touched.entity, owner);
    assert!((touched.point[1] - 1.5).abs() < 0.001);
    let mut snapshot = input(owner, &sections, &slab, 1);
    snapshot.pose.position = [2.0, 0.0, 0.0];
    let report = cache
        .synchronize(&world, &mut physics, &[snapshot])
        .unwrap();
    assert_eq!((report.compiled, report.rebuilt), (0, 1));
    assert!(hit(&physics, 3.5, 0.5).is_some());
    let mut snapshot = input(owner, &sections, &slab, 1);
    snapshot.pose.position = [3.0, 0.0, 0.0];
    let report = cache
        .synchronize(&world, &mut physics, &[snapshot])
        .unwrap();
    assert_eq!((report.compiled, report.rebuilt), (0, 0));
    assert!(hit(&physics, 4.5, 0.5).is_some());
    let mut snapshot = input(owner, &sections, &slab, 1);
    snapshot.settings.sensor = true;
    let report = cache
        .synchronize(&world, &mut physics, &[snapshot])
        .unwrap();
    assert_eq!((report.compiled, report.rebuilt), (0, 1));
    assert!(hit(&physics, 1.5, 0.5).is_none());
    assert!(
        physics
            .raycast(
                [1.5, 8.0, 0.5],
                [0.0, -1.0, 0.0],
                20.0,
                RaycastFilter3d {
                    include_sensors: true,
                    ..RaycastFilter3d::default()
                }
            )
            .unwrap()
            .is_some()
    );
}

#[test]
fn late_invalid_inputs_and_budget_excess_preserve_the_complete_previous_snapshot() {
    let mut world = World::default();
    let owner = world.spawn(EntityData::default());
    let second = world.spawn(EntityData::default());
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
    let empty = VoxelSection::default();
    let edited = [VoxelCollisionSection3d {
        coord: SectionCoord::default(),
        revision: 2,
        voxels: &empty,
    }];
    let bad = |_| {
        Some(VoxelShape {
            min: [0; 3],
            max: [0; 3],
        })
    };
    assert!(
        cache
            .synchronize(
                &world,
                &mut physics,
                &[
                    input(owner, &edited, &solid, 1),
                    input(second, &sections, &bad, 1)
                ]
            )
            .is_err()
    );
    assert_eq!(hit(&physics, 0.5, 0.5).unwrap().entity, owner);
    assert_eq!(cache.next_key, 1);
    assert_eq!(
        cache.owners[&owner].sections[&SectionCoord::default()].revision,
        1
    );
    for budget in [
        VoxelCollisionBudget3d {
            worlds: 0,
            ..VoxelCollisionBudget3d::default()
        },
        VoxelCollisionBudget3d {
            sections: 0,
            ..VoxelCollisionBudget3d::default()
        },
        VoxelCollisionBudget3d {
            rebuilds: 0,
            ..VoxelCollisionBudget3d::default()
        },
        VoxelCollisionBudget3d {
            pieces: 0,
            ..VoxelCollisionBudget3d::default()
        },
    ] {
        cache.budget = budget;
        assert!(
            cache
                .synchronize(&world, &mut physics, &[input(owner, &sections, &solid, 2)])
                .is_err()
        );
        assert_eq!(hit(&physics, 0.5, 0.5).unwrap().entity, owner);
    }
    cache.budget = VoxelCollisionBudget3d::default();
    cache
        .synchronize(&world, &mut physics, &[input(owner, &edited, &solid, 1)])
        .unwrap();
    assert!(hit(&physics, 0.5, 0.5).is_none());
}

#[test]
fn duplicates_bad_empty_settings_and_unrelated_solver_owners_reject_without_mutation() {
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
    assert!(
        cache
            .synchronize(
                &world,
                &mut physics,
                &[
                    input(owner, &sections, &solid, 1),
                    input(owner, &[], &solid, 1)
                ]
            )
            .is_err()
    );
    let duplicated = [
        VoxelCollisionSection3d {
            coord: sections[0].coord,
            revision: 1,
            voxels: &voxels,
        },
        VoxelCollisionSection3d {
            coord: sections[0].coord,
            revision: 1,
            voxels: &voxels,
        },
    ];
    assert!(
        cache
            .synchronize(
                &world,
                &mut physics,
                &[input(owner, &duplicated, &solid, 1)]
            )
            .is_err()
    );
    let mut snapshot = input(owner, &[], &solid, 1);
    snapshot.scale = [0.0; 3];
    assert!(
        cache
            .synchronize(&world, &mut physics, &[snapshot])
            .is_err()
    );
    let mut snapshot = input(owner, &[], &solid, 1);
    snapshot.settings.friction = -1.0;
    assert!(
        cache
            .synchronize(&world, &mut physics, &[snapshot])
            .is_err()
    );
    assert!(!physics.contains(owner));
    physics
        .insert_body(owner, RigidBody3d::default(), &[Collider3d::sphere(0.5)])
        .unwrap();
    assert!(
        cache
            .synchronize(&world, &mut physics, &[input(owner, &sections, &solid, 1)])
            .is_err()
    );
    assert!(physics.contains(owner));
}

#[path = "lifecycle_tests.rs"]
mod lifecycle;
