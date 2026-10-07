//! Differential candidate coverage and deterministic sparse-world work counts.

use std::{cell::Cell, time::Duration};

use super::*;
use crate::{
    Collider3d, ColliderShape3d, PhysicsPose3d, RaycastFilter3d, RigidBody3d, RigidBodyKind,
};

fn id(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}
fn at(position: [f32; 3], rotation: [f32; 4]) -> PhysicsPose3d {
    PhysicsPose3d { position, rotation }
}
const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

fn populated() -> PhysicsWorld3d {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let half = std::f32::consts::FRAC_1_SQRT_2;
    for i in (1..=120_u16).rev() {
        let position = [
            f32::from(i % 10) * 3.0 - 12.0,
            f32::from((i / 10) % 4) * 3.0 - 5.0,
            f32::from(i / 40) * 3.0 - 3.0,
        ];
        let mut pieces = [
            Collider3d::sphere(0.5),
            Collider3d::cuboid([0.8, 0.2, 0.4]),
            Collider3d::capsule(0.4, 0.2),
        ];
        pieces[0].sensor = i % 3 == 0;
        pieces[0].layers.memberships = 2;
        pieces[1].offset = [0.4, 0.7, -0.3];
        pieces[1].rotation = [0.0, 0.0, half, half];
        pieces[2].offset = [-0.5, 0.0, 0.5];
        world
            .insert_static_collider(
                id(u32::from(i)),
                at(position, [0.0, half, 0.0, half]),
                &pieces,
            )
            .unwrap();
    }
    world
}

fn with_scan<T>(world: &mut PhysicsWorld3d, read: impl FnOnce(&PhysicsWorld3d) -> T) -> T {
    let mut scan = SpatialIndex::default();
    scan.pieces.clone_from(&world.spatial.pieces);
    scan.unbounded.extend(scan.pieces.keys().copied());
    std::mem::swap(&mut world.spatial, &mut scan);
    let result = read(world);
    std::mem::swap(&mut world.spatial, &mut scan);
    result
}

// Force every registered piece into the exact phase to catch omitted candidates.
fn compare_scan(world: &mut PhysicsWorld3d) {
    let half = std::f32::consts::FRAC_1_SQRT_2;
    let mut observed = [false; 3];
    for sensors in [false, true] {
        for mask in [0, 1, 2, u32::MAX] {
            for i in 0..15_u16 {
                let filter = RaycastFilter3d {
                    mask,
                    include_sensors: sensors,
                    exclude: Some(id(3)),
                };
                let pose = at(
                    [
                        f32::from(i) - 9.0,
                        f32::from(i % 4) * 1.5 - 5.0,
                        f32::from(i % 3) * 1.5 - 3.0,
                    ],
                    [half, 0.0, 0.0, half],
                );
                let shape = match i % 3 {
                    0 => ColliderShape3d::Sphere { radius: 0.7 },
                    1 => ColliderShape3d::Box {
                        half_extents: [0.4, 0.7, 0.2],
                    },
                    _ => ColliderShape3d::Capsule {
                        half_height: 0.5,
                        radius: 0.3,
                    },
                };
                let direction = [0.7, -0.2, 0.3];
                let collect = |world: &PhysicsWorld3d| {
                    (
                        world
                            .raycast_where(pose.position, direction, 35.0, filter, |entity| {
                                entity != id(4)
                            })
                            .unwrap(),
                        world
                            .overlap_where(shape, pose, filter, |entity| entity != id(4))
                            .unwrap(),
                        world
                            .shape_cast_where(shape, pose, direction, 35.0, filter, |entity| {
                                entity != id(4)
                            })
                            .unwrap(),
                    )
                };
                let indexed = collect(world);
                observed[0] |= indexed.0.is_some();
                observed[1] |= !indexed.1.is_empty();
                observed[2] |= indexed.2.is_some();
                let exhaustive = with_scan(world, collect);
                assert_eq!(
                    indexed, exhaustive,
                    "mask {mask}, sensors {sensors}, probe {i}"
                );
            }
        }
    }
    assert!(
        observed.into_iter().all(|hit| hit),
        "differential probes must exercise hits"
    );
}

#[test]
fn indexed_queries_match_scan_before_step_after_teleport_remove_and_reinsert() {
    let mut world = populated();
    compare_scan(&mut world);
    let half = std::f32::consts::FRAC_1_SQRT_2;
    world
        .move_to(id(8), at([0.0; 3], [half, 0.0, 0.0, half]))
        .unwrap();
    assert!(world.remove(id(17)));
    assert!(world.remove(id(18)));
    world
        .insert_static_collider(id(200), at([1.0; 3], IDENTITY), &[Collider3d::sphere(2.0)])
        .unwrap();
    compare_scan(&mut world);
}

#[test]
fn solved_motion_rotation_and_pending_targets_refresh_only_at_the_right_time() {
    let mut world = populated();
    let half = std::f32::consts::FRAC_1_SQRT_2;
    for (entity, kind) in [
        (id(201), RigidBodyKind::KinematicVelocity),
        (id(202), RigidBodyKind::KinematicPosition),
        (id(203), RigidBodyKind::Dynamic),
    ] {
        world
            .insert_body(
                entity,
                RigidBody3d {
                    kind,
                    position: [30.0, 0.0, 0.0],
                    ..RigidBody3d::default()
                },
                &[Collider3d {
                    offset: [0.0, 0.0, 2.0],
                    ..Collider3d::cuboid([1.0, 0.1, 0.1])
                }],
            )
            .unwrap();
    }
    world.set_linear_velocity(id(201), [4.0, 2.0, 3.0]).unwrap();
    world
        .set_angular_velocity(id(201), [0.0, 1.0, 0.0])
        .unwrap();
    world
        .set_linear_velocity(id(203), [0.0, 2.0, -3.0])
        .unwrap();
    world
        .set_kinematic_target(id(202), at([25.0, 0.0, 0.0], [0.0, half, 0.0, half]))
        .unwrap();
    let probe = ColliderShape3d::Sphere { radius: 0.2 };
    let filter = RaycastFilter3d::default();
    assert!(
        world
            .overlap(probe, at([27.0, 0.0, 0.0], IDENTITY), filter)
            .unwrap()
            .is_empty()
    );
    world.step(Duration::from_secs_f32(0.5)).unwrap();
    assert_eq!(
        world
            .overlap(probe, at([27.0, 0.0, 0.0], IDENTITY), filter)
            .unwrap(),
        vec![id(202)]
    );
    for entity in [id(201), id(203)] {
        let body = &world.backend.bodies[world.bodies[&entity].body];
        let point = body
            .position()
            .transform_point(r3::Vector::new(0.0, 0.0, 2.0));
        assert!(
            world
                .overlap(probe, at(point.to_array(), IDENTITY), filter)
                .unwrap()
                .contains(&entity)
        );
    }
    compare_scan(&mut world);
    world.remove(id(201));
    assert!(!world.spatial.moving.contains(&id(201)));
}

fn sparse_world(count: u16) -> PhysicsWorld3d {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    for i in 0..count {
        let value = f32::from(i) * 4.0;
        world
            .insert_static_collider(
                id(u32::from(i)),
                at([value; 3], IDENTITY),
                &[Collider3d::sphere(0.5)],
            )
            .unwrap();
    }
    world
}

#[test]
fn sparse_xyz_queries_select_one_piece_as_the_world_grows() {
    for count in [100_u16, 1_000, 10_000] {
        let world = sparse_world(count);
        for sample in [0, count / 2, count - 1] {
            let value = f32::from(sample) * 4.0;
            for axis in 0..3 {
                let mut origin = [value; 3];
                origin[axis] -= 1.0;
                let mut direction = [0.0; 3];
                direction[axis] = 1.0;
                let ray = Ray::new(
                    r3::Vector::from_array(origin),
                    r3::Vector::from_array(direction),
                );
                let visited = Cell::new(0_u32);
                let candidates = world
                    .spatial
                    .tree
                    .leaves(|node| {
                        visited.set(visited.get() + 1);
                        node.aabb().cast_local_ray(&ray, 2.0, true).is_some()
                    })
                    .count();
                assert_eq!(candidates, 1);
                assert!(
                    visited.get() < u32::from(count) / 2,
                    "{count}: {} nodes",
                    visited.get()
                );
                assert_eq!(world.spatial.ray(&ray, 2.0).len(), 1);
                let calls = Cell::new(0);
                assert_eq!(
                    world
                        .raycast_where(origin, direction, 2.0, RaycastFilter3d::default(), |_| {
                            calls.set(calls.get() + 1);
                            true
                        })
                        .unwrap()
                        .unwrap()
                        .entity,
                    id(u32::from(sample))
                );
                assert_eq!(calls.get(), 1);
                eprintln!(
                    "{count} pieces, sample {sample}, axis {axis}: {} nodes, 1 candidate",
                    visited.get()
                );
            }
            let area = Aabb::new(
                r3::Vector::splat(value - 0.25),
                r3::Vector::splat(value + 0.25),
            );
            assert_eq!(world.spatial.area(area).len(), 1);
            let shape = ColliderShape3d::Sphere { radius: 0.2 };
            let calls = Cell::new(0);
            assert_eq!(
                world
                    .overlap_where(
                        shape,
                        at([value; 3], IDENTITY),
                        RaycastFilter3d::default(),
                        |_| {
                            calls.set(calls.get() + 1);
                            true
                        }
                    )
                    .unwrap(),
                vec![id(u32::from(sample))]
            );
            assert_eq!(calls.get(), 1);
            let calls = Cell::new(0);
            assert_eq!(
                world
                    .shape_cast_where(
                        shape,
                        at([value - 1.0, value, value], IDENTITY),
                        [1.0, 0.0, 0.0],
                        2.0,
                        RaycastFilter3d::default(),
                        |_| {
                            calls.set(calls.get() + 1);
                            true
                        }
                    )
                    .unwrap()
                    .unwrap()
                    .entity,
                id(u32::from(sample))
            );
            assert_eq!(calls.get(), 1);
        }
        assert!(
            world.spatial.moving.is_empty(),
            "static terrain needs no step refresh"
        );
    }
}

#[test]
fn overflowing_piece_and_probe_bounds_use_the_conservative_scan_fallback() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_static_collider(
            id(1),
            at([f32::MAX, 0.0, 0.0], IDENTITY),
            &[Collider3d::sphere(f32::MAX)],
        )
        .unwrap();
    assert_eq!(world.spatial.unbounded.len(), 1);
    world
        .insert_static_collider(id(2), at([0.0; 3], IDENTITY), &[Collider3d::sphere(0.5)])
        .unwrap();
    let bounds = Aabb::new(r3::Vector::splat(-f32::MAX), r3::Vector::splat(f32::MAX));
    assert_eq!(world.spatial.area(bounds).len(), 2);
    // This tests candidate retention, not backend geometry at f32::MAX, where
    // exact shape casts can return non-finite witnesses/time of impact.
    let ray = Ray::new(r3::Vector::ZERO, r3::Vector::X);
    assert_eq!(world.spatial.ray(&ray, 1.0)[0].entity, id(1));
    world.remove(id(1));
    assert!(world.spatial.unbounded.is_empty());
    assert_eq!(world.spatial.pieces.len(), 1);
    world.remove(id(2));
    assert!(world.spatial.pieces.is_empty());
}
