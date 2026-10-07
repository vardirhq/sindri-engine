//! Exact 3D query contract before host integration or spatial acceleration.

use sindri_core::EntityId;
use sindri_physics::{
    Collider3d, ColliderShape3d, CollisionLayers, PhysicsError, PhysicsPose3d, PhysicsWorld3d,
    RaycastFilter3d, RigidBody3d, RigidBodyKind,
};
use std::time::Duration;

fn id(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}
fn at(position: [f32; 3]) -> PhysicsPose3d {
    PhysicsPose3d {
        position,
        ..PhysicsPose3d::default()
    }
}
fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.002, "{actual} != {expected}");
}
fn near3(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        near(actual, expected);
    }
}
fn ray(world: &PhysicsWorld3d, filter: RaycastFilter3d) -> Option<sindri_physics::RayHit3d> {
    world
        .raycast([0.0; 3], [1.0, 0.0, 0.0], 10.0, filter)
        .unwrap()
}

#[test]
fn rays_hit_all_shapes_on_every_axis_with_world_points_and_normals() {
    for collider in [
        Collider3d::sphere(0.5),
        Collider3d::cuboid([0.5; 3]),
        Collider3d::capsule(0.5, 0.5),
    ] {
        for axis in 0..3 {
            let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
            let mut position = [0.0; 3];
            position[axis] = 3.0;
            let mut direction = [0.0; 3];
            direction[axis] = 7.0;
            world
                .insert_static_collider(id(1), at(position), &[collider])
                .unwrap();
            let hit = world
                .raycast([0.0; 3], direction, 5.0, RaycastFilter3d::default())
                .unwrap()
                .unwrap();
            let extent = if axis == 1 && matches!(collider.shape, ColliderShape3d::Capsule { .. }) {
                1.0
            } else {
                0.5
            };
            near(hit.distance, 3.0 - extent);
            let mut point = [0.0; 3];
            point[axis] = 3.0 - extent;
            let mut normal = [0.0; 3];
            normal[axis] = -1.0;
            near3(hit.point, point);
            near3(hit.normal, normal);
            assert_eq!(hit.entity, id(1));
        }
    }
}

fn filtering_world() -> PhysicsWorld3d {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    for index in [8, 2] {
        let piece = Collider3d {
            layers: CollisionLayers::new(2, 0),
            ..Collider3d::sphere(0.5)
        };
        world
            .insert_static_collider(id(index), at([3.0, 0.0, 0.0]), &[piece, piece])
            .unwrap();
    }
    world
        .insert_static_collider(
            id(1),
            at([1.0, 0.0, 0.0]),
            &[Collider3d {
                sensor: true,
                ..Collider3d::sphere(0.2)
            }],
        )
        .unwrap();
    world
}

#[test]
fn filtering_ties_and_compound_predicates_are_shared_by_every_query() {
    let world = filtering_world();
    let filter = RaycastFilter3d::default();
    assert_eq!(ray(&world, filter).unwrap().entity, id(2));
    assert_eq!(
        ray(
            &world,
            RaycastFilter3d {
                include_sensors: true,
                ..filter
            }
        )
        .unwrap()
        .entity,
        id(1)
    );
    assert!(ray(&world, RaycastFilter3d { mask: 0, ..filter }).is_none());
    assert!(ray(&world, RaycastFilter3d { mask: 1, ..filter }).is_none());
    assert_eq!(
        ray(
            &world,
            RaycastFilter3d {
                exclude: Some(id(2)),
                ..filter
            }
        )
        .unwrap()
        .entity,
        id(8)
    );
    let probe = ColliderShape3d::Sphere { radius: 0.2 };
    let mut visits = Vec::new();
    let hit = world
        .raycast_where([0.0; 3], [1.0, 0.0, 0.0], 10.0, filter, |entity| {
            visits.push(entity);
            entity != id(2)
        })
        .unwrap()
        .unwrap();
    assert_eq!(hit.entity, id(8));
    assert_eq!(visits, vec![id(1), id(2), id(8)]);
    let mut visits = Vec::new();
    let hits = world
        .overlap_where(probe, at([3.0, 0.0, 0.0]), filter, |entity| {
            visits.push(entity);
            true
        })
        .unwrap();
    assert_eq!(hits, vec![id(2), id(8)]);
    assert_eq!(visits, vec![id(1), id(2), id(8)]);
    let hit = world
        .shape_cast_where(
            probe,
            at([0.0; 3]),
            [1.0, 0.0, 0.0],
            10.0,
            filter,
            |entity| entity != id(2),
        )
        .unwrap()
        .unwrap();
    assert_eq!(hit.entity, id(8));
    near(hit.distance, 2.3);
    assert_eq!(
        world
            .shape_cast(probe, at([0.0; 3]), [1.0, 0.0, 0.0], 10.0, filter)
            .unwrap()
            .unwrap()
            .entity,
        id(2)
    );
    assert!(
        world
            .overlap(
                probe,
                at([3.0, 0.0, 0.0]),
                RaycastFilter3d { mask: 0, ..filter }
            )
            .unwrap()
            .is_empty()
    );
    assert!(
        world
            .shape_cast(
                probe,
                at([0.0; 3]),
                [1.0, 0.0, 0.0],
                10.0,
                RaycastFilter3d { mask: 0, ..filter }
            )
            .unwrap()
            .is_none()
    );
}

#[test]
fn queries_observe_insert_teleport_solved_and_pending_poses_and_removal() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let filter = RaycastFilter3d::default();
    world
        .insert_body(
            id(1),
            RigidBody3d {
                kind: RigidBodyKind::KinematicPosition,
                position: [3.0, 0.0, 0.0],
                ..RigidBody3d::default()
            },
            &[Collider3d::sphere(0.5)],
        )
        .unwrap();
    near(ray(&world, filter).unwrap().distance, 2.5);
    world
        .set_kinematic_target(id(1), at([7.0, 0.0, 0.0]))
        .unwrap();
    near(ray(&world, filter).unwrap().distance, 2.5);
    world.step(Duration::from_millis(10)).unwrap();
    near(ray(&world, filter).unwrap().distance, 6.5);
    assert!(world.remove(id(1)));
    assert!(ray(&world, filter).is_none());
    world
        .insert_body(
            id(2),
            RigidBody3d {
                kind: RigidBodyKind::KinematicVelocity,
                position: [3.0, 0.0, 0.0],
                linear_velocity: [1.0, 0.0, 0.0],
                ..RigidBody3d::default()
            },
            &[Collider3d::sphere(0.5)],
        )
        .unwrap();
    world.step(Duration::from_secs(1)).unwrap();
    near(ray(&world, filter).unwrap().distance, 3.5);
    world.move_to(id(2), at([1.0, 0.0, 0.0])).unwrap();
    near(ray(&world, filter).unwrap().distance, 0.5);
    assert_eq!(ray(&world, filter).unwrap().entity, id(2));
    let probe = ColliderShape3d::Sphere { radius: 0.1 };
    assert_eq!(
        world.overlap(probe, at([1.0, 0.0, 0.0]), filter).unwrap(),
        vec![id(2)]
    );
    near(
        world
            .shape_cast(probe, at([0.0; 3]), [1.0, 0.0, 0.0], 5.0, filter)
            .unwrap()
            .unwrap()
            .distance,
        0.4,
    );
}

#[test]
fn body_local_and_probe_quaternions_control_exact_geometry() {
    let half = std::f32::consts::FRAC_1_SQRT_2;
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    // Body turns local Z offset into world X; piece turns its long X axis upright.
    world
        .insert_static_collider(
            id(1),
            PhysicsPose3d {
                rotation: [0.0, half, 0.0, half],
                ..at([0.0; 3])
            },
            &[Collider3d {
                offset: [0.0, 0.0, 3.0],
                rotation: [0.0, 0.0, half, half],
                ..Collider3d::cuboid([2.0, 0.25, 0.25])
            }],
        )
        .unwrap();
    let filter = RaycastFilter3d::default();
    near(ray(&world, filter).unwrap().distance, 2.75);
    let probe = ColliderShape3d::Box {
        half_extents: [2.0, 0.1, 0.1],
    };
    assert_eq!(
        world.overlap(probe, at([1.0, 0.0, 0.0]), filter).unwrap(),
        vec![id(1)]
    );
    let rotated = PhysicsPose3d {
        rotation: [0.0, 0.0, half, half],
        ..at([1.0, 0.0, 0.0])
    };
    assert!(world.overlap(probe, rotated, filter).unwrap().is_empty());
    near(
        world
            .shape_cast(probe, rotated, [1.0, 0.0, 0.0], 5.0, filter)
            .unwrap()
            .unwrap()
            .distance,
        1.65,
    );
}

#[test]
fn sweeps_cover_all_probe_shapes_world_normals_and_initial_overlap() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_static_collider(
            id(1),
            at([0.0, 0.0, 3.0]),
            &[Collider3d::cuboid([1.0, 1.0, 0.5])],
        )
        .unwrap();
    let filter = RaycastFilter3d::default();
    for probe in [
        ColliderShape3d::Sphere { radius: 0.25 },
        ColliderShape3d::Box {
            half_extents: [0.25; 3],
        },
        ColliderShape3d::Capsule {
            half_height: 0.5,
            radius: 0.25,
        },
        ColliderShape3d::Capsule {
            half_height: 0.0,
            radius: 0.25,
        },
    ] {
        let hit = world
            .shape_cast(probe, at([0.0; 3]), [0.0, 0.0, 8.0], 5.0, filter)
            .unwrap()
            .unwrap();
        near(hit.distance, 2.25);
        near3(hit.normal, [0.0, 0.0, -1.0]);
        near(hit.point[2], 2.5);
        assert!(
            world
                .shape_cast(probe, at([0.0; 3]), [0.0, 0.0, 1.0], 2.0, filter)
                .unwrap()
                .is_none()
        );
        let inside = world
            .shape_cast(probe, at([0.0, 0.0, 3.0]), [1.0, 0.0, 0.0], 0.0, filter)
            .unwrap()
            .unwrap();
        near(inside.distance, 0.0);
        near3(inside.normal, [0.0; 3]);
        near3(inside.point, [0.0, 0.0, 3.0]);
        assert_eq!(
            world.overlap(probe, at([0.0, 0.0, 3.0]), filter).unwrap(),
            vec![id(1)]
        );
    }
    let inside = world
        .raycast([0.0, 0.0, 3.0], [1.0, 0.0, 0.0], 0.0, filter)
        .unwrap()
        .unwrap();
    near(inside.distance, 0.0);
    near3(inside.normal, [0.0; 3]);
    near3(inside.point, [0.0, 0.0, 3.0]);
}

#[test]
fn rays_accept_extreme_directions_and_inclusive_endpoints() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_static_collider(id(1), at([3.0, 0.0, 0.0]), &[Collider3d::cuboid([0.5; 3])])
        .unwrap();
    for magnitude in [f32::MAX, f32::from_bits(1)] {
        let hit = world
            .raycast(
                [0.0; 3],
                [magnitude, 0.0, 0.0],
                2.5,
                RaycastFilter3d::default(),
            )
            .unwrap()
            .unwrap();
        near(hit.distance, 2.5);
        let hit = world
            .shape_cast(
                ColliderShape3d::Sphere { radius: 0.25 },
                at([0.0; 3]),
                [magnitude, 0.0, 0.0],
                4.0,
                RaycastFilter3d::default(),
            )
            .unwrap()
            .unwrap();
        near(hit.distance, 2.25);
    }
}

#[test]
fn invalid_queries_fail_before_visiting_entities_and_keep_world_unchanged() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_static_collider(id(1), at([3.0, 0.0, 0.0]), &[Collider3d::sphere(0.5)])
        .unwrap();
    let filter = RaycastFilter3d::default();
    for (origin, direction, distance) in [
        ([f32::NAN, 0.0, 0.0], [1.0, 0.0, 0.0], 1.0),
        ([0.0; 3], [f32::INFINITY, 0.0, 0.0], 1.0),
        ([0.0; 3], [0.0; 3], 1.0),
        ([0.0; 3], [1.0, 0.0, 0.0], -1.0),
        ([0.0; 3], [1.0, 0.0, 0.0], f32::NAN),
        ([f32::MAX, 0.0, 0.0], [1.0, 0.0, 0.0], f32::MAX),
    ] {
        assert!(
            world
                .raycast_where(origin, direction, distance, filter, |_| panic!("visited"))
                .is_err()
        );
        assert!(
            world
                .shape_cast_where(
                    ColliderShape3d::Sphere { radius: 0.25 },
                    at(origin),
                    direction,
                    distance,
                    filter,
                    |_| panic!("visited")
                )
                .is_err()
        );
    }
    for shape in [
        ColliderShape3d::Sphere { radius: 0.0 },
        ColliderShape3d::Box {
            half_extents: [0.5, -1.0, 0.5],
        },
        ColliderShape3d::Capsule {
            half_height: -1.0,
            radius: 0.5,
        },
    ] {
        assert!(
            world
                .overlap_where(shape, at([0.0; 3]), filter, |_| panic!("visited"))
                .is_err()
        );
    }
    let invalid = PhysicsPose3d {
        rotation: [0.0; 4],
        ..at([0.0; 3])
    };
    assert_eq!(
        world.overlap(ColliderShape3d::Sphere { radius: 0.5 }, invalid, filter),
        Err(PhysicsError::InvalidQuaternion("rotation"))
    );
    assert!(
        world
            .shape_cast(
                ColliderShape3d::Sphere { radius: 0.5 },
                invalid,
                [1.0, 0.0, 0.0],
                5.0,
                filter
            )
            .is_err()
    );
    near(ray(&world, filter).unwrap().distance, 2.5);
}

#[test]
fn equal_distance_pieces_keep_authored_order_for_the_surface_normal() {
    let sphere = Collider3d {
        offset: [3.0, 1.0, 0.0],
        ..Collider3d::sphere(1.0)
    };
    let cube = Collider3d {
        offset: [3.5, 0.0, 0.0],
        ..Collider3d::cuboid([0.5; 3])
    };
    for sphere_first in [true, false] {
        let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
        let pieces = if sphere_first {
            [sphere, cube]
        } else {
            [cube, sphere]
        };
        world
            .insert_static_collider(id(1), at([0.0; 3]), &pieces)
            .unwrap();
        let hit = ray(&world, RaycastFilter3d::default()).unwrap();
        near(hit.distance, 3.0);
        near3(
            hit.normal,
            if sphere_first {
                [0.0, -1.0, 0.0]
            } else {
                [-1.0, 0.0, 0.0]
            },
        );
    }
}
