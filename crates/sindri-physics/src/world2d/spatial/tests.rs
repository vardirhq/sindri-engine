//! Differential broad-phase coverage and deterministic work-count scaling.

use std::{cell::Cell, time::Duration};

use super::*;
use crate::{
    Collider2d, ColliderShape2d, PhysicsPose2d, RaycastFilter2d, RigidBody2d, RigidBodyKind,
};

fn id(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}

fn pose(x: f32, y: f32, rotation: f32) -> PhysicsPose2d {
    PhysicsPose2d {
        position: [x, y],
        rotation,
    }
}

fn populated() -> PhysicsWorld2d {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    // Reverse insertion order, duplicate geometry, sensors, offsets and rotations.
    for i in (1..=80_u16).rev() {
        let x = f32::from(i % 10) * 3.0 - 12.0;
        let y = f32::from(i / 10) * 3.0 - 12.0;
        let mut pieces = vec![Collider2d::circle(0.5), Collider2d::rectangle([0.8, 0.2])];
        pieces[0].sensor = i % 3 == 0;
        pieces[0].layers.memberships = 2;
        pieces[1].offset = [0.4, 0.7];
        pieces[1].rotation = 0.6;
        world
            .insert_static_collider(id(u32::from(i)), pose(x, y, 0.3), &pieces)
            .unwrap();
    }
    world
}

// Same exact narrow phase with all registered pieces forced onto the scan path.
// Comparing this with the BVH catches omitted candidates without a timing oracle.
fn compare_scan(world: &mut PhysicsWorld2d) {
    for sensors in [false, true] {
        for mask in [0, 1, 2, u32::MAX] {
            for i in 0..15_u16 {
                let filter = RaycastFilter2d {
                    mask,
                    include_sensors: sensors,
                    exclude: Some(id(3)),
                };
                let at = pose(f32::from(i) - 9.0, f32::from(i) * 0.7 - 7.0, 0.43);
                let shape = ColliderShape2d::Capsule {
                    half_height: 0.5,
                    radius: 0.3,
                };
                let collect = |world: &PhysicsWorld2d| {
                    (
                        world
                            .raycast_where(at.position, [0.7, -0.2], 35.0, filter, |entity| {
                                entity != id(4)
                            })
                            .unwrap(),
                        world
                            .overlap_where(shape, at, filter, |entity| entity != id(4))
                            .unwrap(),
                        world
                            .shape_cast_where(shape, at, [0.7, -0.2], 35.0, filter, |entity| {
                                entity != id(4)
                            })
                            .unwrap(),
                    )
                };
                let indexed = collect(world);
                let mut scan = SpatialIndex::default();
                scan.pieces.clone_from(&world.spatial.pieces);
                scan.unbounded.extend(scan.pieces.keys().copied());
                std::mem::swap(&mut world.spatial, &mut scan);
                let exhaustive = collect(world);
                std::mem::swap(&mut world.spatial, &mut scan);
                assert_eq!(
                    indexed, exhaustive,
                    "mask {mask}, sensors {sensors}, probe {i}"
                );
            }
        }
    }
}

#[test]
fn indexed_queries_match_scan_before_step_after_teleport_remove_and_reinsert() {
    let mut world = populated();
    compare_scan(&mut world);
    world.move_to(id(8), pose(0.0, 0.0, 1.2)).unwrap();
    assert!(world.remove(id(17)));
    assert!(world.remove(id(18)));
    // Reuses a backend arena slot without retaining the removed generation.
    world
        .insert_static_collider(id(100), pose(1.0, 1.0, 0.0), &[Collider2d::circle(2.0)])
        .unwrap();
    compare_scan(&mut world);
}

#[test]
fn solved_velocity_and_pending_position_targets_keep_their_query_timing() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    for (entity, kind) in [
        (id(1), RigidBodyKind::KinematicVelocity),
        (id(2), RigidBodyKind::KinematicPosition),
    ] {
        world
            .insert_body(
                entity,
                RigidBody2d {
                    kind,
                    ..RigidBody2d::default()
                },
                &[Collider2d::circle(0.5)],
            )
            .unwrap();
    }
    world.set_linear_velocity(id(1), [4.0, 0.0]).unwrap();
    world
        .set_kinematic_target(id(2), pose(3.0, 0.0, 0.0))
        .unwrap();
    let filter = RaycastFilter2d::default();
    assert!(
        world
            .overlap(
                ColliderShape2d::Circle { radius: 0.1 },
                pose(3.0, 0.0, 0.0),
                filter
            )
            .unwrap()
            .is_empty()
    );
    world.step(Duration::from_secs_f32(0.5)).unwrap();
    assert_eq!(
        world
            .overlap(
                ColliderShape2d::Circle { radius: 0.1 },
                pose(3.0, 0.0, 0.0),
                filter
            )
            .unwrap(),
        vec![id(2)]
    );
    compare_scan(&mut world);
}

#[test]
fn sparse_queries_select_one_piece_as_the_world_grows() {
    for count in [100_u16, 1_000, 10_000] {
        let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
        for i in 0..count {
            world
                .insert_static_collider(
                    id(u32::from(i)),
                    pose(f32::from(i) * 4.0, 0.0, 0.0),
                    &[Collider2d::circle(0.5)],
                )
                .unwrap();
        }
        for sample in [0, count / 2, count - 1] {
            let x = f32::from(sample) * 4.0;
            let ray = Ray::new(r2::Vector::new(x - 1.0, 0.0), r2::Vector::X);
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
                "{count}: visited {} nodes",
                visited.get()
            );
            assert_eq!(world.spatial.ray(&ray, 2.0).len(), 1);
            let area = Aabb::new(
                r2::Vector::new(x - 0.25, -0.25),
                r2::Vector::new(x + 0.25, 0.25),
            );
            assert_eq!(world.spatial.area(area).len(), 1);
            eprintln!(
                "{count} pieces, sample {sample}: {} BVH nodes, 1 candidate",
                visited.get()
            );
        }
        assert!(
            world.spatial.moving.is_empty(),
            "static terrain needs no step refresh"
        );
        let hit = world
            .raycast([-1.0, 0.0], [1.0, 0.0], 2.0, RaycastFilter2d::default())
            .unwrap()
            .unwrap();
        assert_eq!(hit.entity, id(0));
    }
}

#[test]
fn overflowing_bounds_use_the_conservative_fallback() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    world
        .insert_static_collider(
            id(1),
            pose(f32::MAX, 0.0, 0.0),
            &[Collider2d::circle(f32::MAX)],
        )
        .unwrap();
    assert_eq!(world.spatial.unbounded.len(), 1);
    compare_scan(&mut world);
    world.remove(id(1));
    assert!(world.spatial.unbounded.is_empty());
}
