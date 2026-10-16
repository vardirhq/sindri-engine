//! Classified support before scene platform ownership is introduced.
use sindri_core::EntityId;
use sindri_physics::{
    CharacterOptions3d, Collider3d, GroundOptions3d, PhysicsPose3d, PhysicsWorld3d, RaycastFilter3d,
};

fn id(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}
fn at(position: [f32; 3]) -> PhysicsPose3d {
    PhysicsPose3d {
        position,
        ..PhysicsPose3d::default()
    }
}
fn floor(world: &mut PhysicsWorld3d, index: u32, height: f32) {
    world
        .insert_static_collider(
            id(index),
            at([0.0, height - 0.25, 0.0]),
            &[Collider3d::cuboid([10.0, 0.25, 10.0])],
        )
        .unwrap();
}
fn probe(
    world: &PhysicsWorld3d,
    collider: Collider3d,
    pose: PhysicsPose3d,
    distance: f32,
) -> sindri_physics::GroundProbe3d {
    world
        .probe_ground(
            collider.shape,
            pose,
            GroundOptions3d {
                max_distance: distance,
                ..GroundOptions3d::default()
            },
            RaycastFilter3d::default(),
        )
        .unwrap()
}

#[test]
fn every_shape_supports_touching_skin_and_downward_travel_without_moving() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    floor(&mut world, 1, 0.0);
    for (collider, extent) in [
        (Collider3d::cuboid([0.25, 0.5, 0.25]), 0.5),
        (Collider3d::sphere(0.5), 0.5),
        (Collider3d::capsule(0.5, 0.25), 0.75),
    ] {
        for gap in [0.0, 0.005, 0.01] {
            let result = probe(&world, collider, at([0.0, extent + gap, 0.0]), 0.0);
            assert!(result.walkable && !result.started_penetrating, "{result:?}");
            let hit = result.hit.unwrap();
            assert!(hit.distance.abs() < f32::EPSILON);
            assert!(hit.normal[1] > 0.99);
            assert!(hit.point[1].abs() < 0.003);
        }
        let pose = at([0.0, extent + 0.2, 0.0]);
        assert!(probe(&world, collider, pose, 0.18).hit.is_none());
        let result = probe(&world, collider, pose, 0.21);
        assert!(result.walkable, "{result:?}");
        assert!((result.hit.unwrap().distance - 0.19).abs() < 0.003);
        assert_eq!(world.pose(id(1)).unwrap(), at([0.0, -0.25, 0.0]));
    }
}

#[test]
fn nearest_steep_support_is_retained_instead_of_probing_through_it() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let angle = std::f32::consts::PI / 6.0;
    let (sin, cos) = (angle * 0.5).sin_cos();
    world
        .insert_static_collider(
            id(1),
            PhysicsPose3d {
                rotation: [0.0, 0.0, sin, cos],
                ..PhysicsPose3d::default()
            },
            &[Collider3d::cuboid([3.0, 0.25, 3.0])],
        )
        .unwrap();
    floor(&mut world, 2, -3.0);
    let pose = at([0.0, 2.0, 0.0]);
    for (limit, expected) in [(0.2, false), (std::f32::consts::FRAC_PI_4, true)] {
        let result = world
            .probe_ground(
                Collider3d::sphere(0.5).shape,
                pose,
                GroundOptions3d {
                    max_distance: 6.0,
                    max_slope_angle: limit,
                    ..GroundOptions3d::default()
                },
                RaycastFilter3d::default(),
            )
            .unwrap();
        assert_eq!(result.hit.unwrap().entity, id(1));
        assert_eq!(result.walkable, expected);
        assert!(!result.started_penetrating);
        assert!((result.hit.unwrap().normal[1] - angle.cos()).abs() < 0.01);
    }
}

#[test]
fn penetration_blocks_support_even_when_a_walkable_floor_is_nearby() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    floor(&mut world, 2, 0.0);
    world
        .insert_static_collider(
            id(1),
            at([0.4, 0.51, 0.0]),
            &[Collider3d::cuboid([0.25; 3])],
        )
        .unwrap();
    let result = probe(&world, Collider3d::sphere(0.5), at([0.0, 0.51, 0.0]), 0.1);
    assert!(result.started_penetrating && !result.walkable);
    assert_eq!(result.hit.unwrap().entity, id(1));
    assert!(
        result
            .hit
            .unwrap()
            .normal
            .into_iter()
            .all(|axis| axis.abs() < f32::EPSILON)
    );
}

#[test]
fn alternate_up_rotated_capsule_and_offset_obstacles_use_world_geometry() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let half = std::f32::consts::FRAC_1_SQRT_2;
    world
        .insert_static_collider(
            id(1),
            PhysicsPose3d {
                rotation: [0.0, 0.0, half, half],
                ..at([3.0, 4.0, 5.0])
            },
            &[Collider3d {
                offset: [0.0, 0.25, 0.0],
                ..Collider3d::cuboid([5.0, 0.25, 5.0])
            }],
        )
        .unwrap();
    let result = world
        .probe_ground(
            Collider3d::capsule(0.5, 0.25).shape,
            PhysicsPose3d {
                rotation: [0.0, 0.0, half, half],
                ..at([3.76, 4.0, 5.0])
            },
            GroundOptions3d {
                up: [1.0, 0.0, 0.0],
                max_distance: 0.0,
                ..GroundOptions3d::default()
            },
            RaycastFilter3d::default(),
        )
        .unwrap();
    assert!(result.walkable, "{result:?}");
    assert!(result.hit.unwrap().normal[0] > 0.99);
}

#[test]
fn masks_sensors_exclusion_and_predicates_apply_once_to_each_entity() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    floor(&mut world, 3, 0.0);
    let sensor = Collider3d {
        sensor: true,
        ..Collider3d::cuboid([5.0, 0.25, 5.0])
    };
    world
        .insert_static_collider(id(1), at([0.0, 0.25, 0.0]), &[sensor, sensor])
        .unwrap();
    let pose = at([0.0, 2.0, 0.0]);
    let options = GroundOptions3d {
        max_distance: 3.0,
        ..GroundOptions3d::default()
    };
    let shape = Collider3d::sphere(0.5).shape;
    let hit = |filter| {
        world
            .probe_ground(shape, pose, options, filter)
            .unwrap()
            .hit
    };
    assert_eq!(hit(RaycastFilter3d::default()).unwrap().entity, id(3));
    assert_eq!(
        hit(RaycastFilter3d {
            include_sensors: true,
            ..RaycastFilter3d::default()
        })
        .unwrap()
        .entity,
        id(1)
    );
    assert!(
        hit(RaycastFilter3d {
            mask: 0,
            ..RaycastFilter3d::default()
        })
        .is_none()
    );
    assert!(
        hit(RaycastFilter3d {
            exclude: Some(id(3)),
            ..RaycastFilter3d::default()
        })
        .is_none()
    );
    let mut visited = Vec::new();
    let result = world
        .probe_ground_where(
            shape,
            pose,
            options,
            RaycastFilter3d {
                include_sensors: true,
                ..RaycastFilter3d::default()
            },
            |entity| {
                assert!(!visited.contains(&entity));
                visited.push(entity);
                entity != id(1)
            },
        )
        .unwrap();
    assert_eq!(result.hit.unwrap().entity, id(3));
}

#[test]
fn exact_ties_current_teleports_removal_and_copies_are_stable() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    floor(&mut world, 5, 0.0);
    floor(&mut world, 2, 0.0);
    let pose = at([0.0, 1.0, 0.0]);
    let snapshot = probe(&world, Collider3d::sphere(0.5), pose, 2.0);
    assert_eq!(snapshot.hit.unwrap().entity, id(2));
    let copied = world.clone();
    assert_eq!(probe(&copied, Collider3d::sphere(0.5), pose, 2.0), snapshot);
    world.move_to(id(2), at([20.0, -0.25, 0.0])).unwrap();
    assert_eq!(
        probe(&world, Collider3d::sphere(0.5), pose, 2.0)
            .hit
            .unwrap()
            .entity,
        id(5)
    );
    world.remove(id(5));
    assert!(
        probe(&world, Collider3d::sphere(0.5), pose, 2.0)
            .hit
            .is_none()
    );
    assert_eq!(snapshot.hit.unwrap().entity, id(2));
}

#[test]
fn classified_support_does_not_inherit_rapiers_broad_ground_prediction() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    floor(&mut world, 1, 0.0);
    let shape = Collider3d::sphere(0.5);
    let pose = at([0.0, 0.54, 0.0]);
    let raw = world
        .move_character(
            shape.shape,
            pose,
            [0.0; 3],
            CharacterOptions3d::default(),
            RaycastFilter3d::default(),
        )
        .unwrap();
    assert!(raw.grounded);
    let classified = probe(&world, shape, pose, 0.0);
    assert!(!classified.walkable && classified.hit.is_none());
    let landing = world
        .move_character(
            shape.shape,
            at([0.0, 2.0, 0.0]),
            [0.0, -3.0, 0.0],
            CharacterOptions3d::default(),
            RaycastFilter3d::default(),
        )
        .unwrap();
    assert!(
        probe(
            &world,
            shape,
            at([0.0, 2.0 + landing.translation[1], 0.0]),
            0.0
        )
        .walkable
    );
}

#[test]
fn invalid_options_fail_before_visiting_candidates() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    floor(&mut world, 1, 0.0);
    let valid = GroundOptions3d::default();
    for options in [
        GroundOptions3d {
            up: [0.0; 3],
            ..valid
        },
        GroundOptions3d {
            up: [0.0, 2.0, 0.0],
            ..valid
        },
        GroundOptions3d {
            up: [f32::NAN; 3],
            ..valid
        },
        GroundOptions3d { skin: 0.0, ..valid },
        GroundOptions3d {
            skin: f32::INFINITY,
            ..valid
        },
        GroundOptions3d {
            max_distance: -1.0,
            ..valid
        },
        GroundOptions3d {
            max_distance: f32::NAN,
            ..valid
        },
        GroundOptions3d {
            max_slope_angle: -0.1,
            ..valid
        },
        GroundOptions3d {
            max_slope_angle: std::f32::consts::PI,
            ..valid
        },
        GroundOptions3d {
            max_slope_angle: f32::NAN,
            ..valid
        },
    ] {
        let mut visited = false;
        assert!(
            world
                .probe_ground_where(
                    Collider3d::sphere(0.5).shape,
                    at([0.0, 1.0, 0.0]),
                    options,
                    RaycastFilter3d::default(),
                    |_| {
                        visited = true;
                        true
                    },
                )
                .is_err()
        );
        assert!(!visited);
    }
}

#[test]
fn invalid_geometry_and_overflow_fail_without_mutation() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    floor(&mut world, 1, 0.0);
    let valid = GroundOptions3d::default();
    for (shape, pose, options) in [
        (Collider3d::sphere(-1.0).shape, at([0.0; 3]), valid),
        (Collider3d::sphere(0.5).shape, at([f32::NAN; 3]), valid),
        (Collider3d::cuboid([f32::MAX; 3]).shape, at([0.0; 3]), valid),
        (
            Collider3d::sphere(0.5).shape,
            at([0.0, -f32::MAX, 0.0]),
            GroundOptions3d {
                max_distance: f32::MAX,
                ..valid
            },
        ),
    ] {
        assert!(
            world
                .probe_ground(shape, pose, options, RaycastFilter3d::default())
                .is_err()
        );
    }
    world
        .insert_static_collider(
            id(2),
            at([f32::MAX, 0.0, 0.0]),
            &[Collider3d::cuboid([f32::MAX, 1.0, 1.0])],
        )
        .unwrap();
    assert!(
        world
            .probe_ground(
                Collider3d::sphere(0.5).shape,
                at([0.0, 1.0, 0.0]),
                valid,
                RaycastFilter3d {
                    exclude: Some(id(2)),
                    ..RaycastFilter3d::default()
                }
            )
            .is_err()
    );
    assert_eq!(world.pose(id(1)).unwrap(), at([0.0, -0.25, 0.0]));
}
