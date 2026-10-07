//! Read-only support classification must not invent grounded state or skip walls.
#[path = "movement/support.rs"]
mod support;
use sindri_physics::{
    Collider2d, ColliderShape2d, GroundOptions2d, PhysicsError, PhysicsPose2d, PhysicsWorld2d,
    RaycastFilter2d,
};
use support::{entity, insert, near, pose};

const PROBE: ColliderShape2d = ColliderShape2d::Circle { radius: 0.5 };

#[test]
fn probing_distinguishes_empty_space_separation_and_touching_support() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    let empty = world
        .probe_ground(
            PROBE,
            pose(0.0, 0.0),
            GroundOptions2d::default(),
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(empty.hit.is_none() && !empty.walkable && !empty.started_penetrating);
    insert(
        &mut world,
        1,
        pose(0.0, -1.0),
        &[Collider2d::rectangle([10.0, 0.5])],
    );
    for (height, travel) in [(0.0, 0.0), (0.005, 0.0), (0.01, 0.0), (0.06, 0.05)] {
        let ground = world
            .probe_ground(
                PROBE,
                pose(0.0, height),
                GroundOptions2d::default(),
                RaycastFilter2d::default(),
            )
            .unwrap();
        assert!(ground.walkable && !ground.started_penetrating);
        let hit = ground.hit.unwrap();
        assert_eq!(hit.entity, entity(1));
        near(hit.distance, travel);
        near(hit.normal[1], 1.0);
        if height <= 0.01 {
            let zero_travel = world
                .probe_ground(
                    PROBE,
                    pose(0.0, height),
                    GroundOptions2d {
                        max_distance: 0.0,
                        ..GroundOptions2d::default()
                    },
                    RaycastFilter2d::default(),
                )
                .unwrap();
            assert!(zero_travel.walkable);
            near(zero_travel.hit.unwrap().distance, 0.0);
        }
    }
    let above = world
        .probe_ground(
            PROBE,
            pose(0.0, 1.0),
            GroundOptions2d::default(),
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(above.hit.is_none());
}

#[test]
fn box_circle_and_rotated_capsule_keep_their_real_support_extent() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(0.0, -1.0),
        &[Collider2d::rectangle([10.0, 0.5])],
    );
    for (shape, rotation, extent) in [
        (PROBE, 0.0, 0.5),
        (
            ColliderShape2d::Box {
                half_extents: [0.4, 0.7],
            },
            0.0,
            0.7,
        ),
        (
            ColliderShape2d::Capsule {
                half_height: 0.4,
                radius: 0.3,
            },
            std::f32::consts::FRAC_PI_2,
            0.3,
        ),
    ] {
        let ground = world
            .probe_ground(
                shape,
                PhysicsPose2d {
                    rotation,
                    ..pose(0.0, extent)
                },
                GroundOptions2d {
                    max_distance: 1.0,
                    ..GroundOptions2d::default()
                },
                RaycastFilter2d::default(),
            )
            .unwrap();
        assert!(ground.walkable);
        near(ground.hit.unwrap().distance, 0.49);
    }
}

#[test]
fn slope_limit_classifies_the_nearest_contact_without_skipping_steep_geometry() {
    for angle in [
        0.0,
        std::f32::consts::FRAC_PI_6,
        std::f32::consts::FRAC_PI_4,
        std::f32::consts::FRAC_PI_3,
    ] {
        let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
        insert(
            &mut world,
            1,
            PhysicsPose2d {
                rotation: angle,
                ..pose(0.0, 0.0)
            },
            &[Collider2d::rectangle([10.0, 0.1])],
        );
        // A walkable floor below the steep obstacle must never replace its hit.
        insert(
            &mut world,
            2,
            pose(0.0, -5.0),
            &[Collider2d::rectangle([10.0, 0.5])],
        );
        let ground = world
            .probe_ground(
                PROBE,
                pose(0.0, 3.0),
                GroundOptions2d {
                    max_distance: 10.0,
                    ..GroundOptions2d::default()
                },
                RaycastFilter2d::default(),
            )
            .unwrap();
        assert_eq!(ground.hit.unwrap().entity, entity(1));
        assert_eq!(ground.walkable, angle <= std::f32::consts::FRAC_PI_4);
        near(ground.hit.unwrap().normal[1], angle.cos());
        assert!(!ground.started_penetrating);
    }
}

#[test]
fn arbitrary_up_uses_world_normals_instead_of_assuming_y_ground() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(-1.0, 0.0),
        &[Collider2d::rectangle([0.5, 10.0])],
    );
    let ground = world
        .probe_ground(
            PROBE,
            pose(0.0, 0.0),
            GroundOptions2d {
                up: [1.0, 0.0],
                ..GroundOptions2d::default()
            },
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(ground.walkable);
    near(ground.hit.unwrap().normal[0], 1.0);
    let mut ceiling = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut ceiling,
        2,
        pose(0.0, 1.0),
        &[Collider2d::rectangle([10.0, 0.5])],
    );
    assert!(
        ceiling
            .probe_ground(
                PROBE,
                pose(0.0, 0.0),
                GroundOptions2d::default(),
                RaycastFilter2d::default()
            )
            .unwrap()
            .hit
            .is_none()
    );
    assert!(
        ceiling
            .probe_ground(
                PROBE,
                pose(0.0, 0.0),
                GroundOptions2d {
                    up: [0.0, -1.0],
                    ..GroundOptions2d::default()
                },
                RaycastFilter2d::default()
            )
            .unwrap()
            .walkable
    );
}

#[test]
fn initial_penetration_is_unwalkable_even_for_zero_probe_travel() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(0.0, 0.0),
        &[Collider2d::rectangle([1.0, 1.0])],
    );
    let ground = world
        .probe_ground(
            PROBE,
            pose(0.0, 0.0),
            GroundOptions2d {
                max_distance: 0.0,
                ..GroundOptions2d::default()
            },
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(ground.started_penetrating && !ground.walkable);
    let hit = ground.hit.unwrap();
    assert_eq!(hit.entity, entity(1));
    near(hit.distance, 0.0);
    near(hit.normal[1], 0.0);
}

#[test]
fn filters_host_predicates_and_current_poses_choose_the_available_support() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    let sensor = Collider2d {
        sensor: true,
        ..Collider2d::rectangle([10.0, 0.5])
    };
    insert(&mut world, 1, pose(0.0, -1.0), &[sensor]);
    insert(
        &mut world,
        2,
        pose(0.0, -2.0),
        &[
            Collider2d::rectangle([10.0, 0.5]),
            Collider2d::rectangle([10.0, 0.5]),
        ],
    );
    insert(
        &mut world,
        3,
        pose(0.0, -3.0),
        &[Collider2d::rectangle([10.0, 0.5])],
    );
    let options = GroundOptions2d {
        max_distance: 5.0,
        ..GroundOptions2d::default()
    };
    let ground = world
        .probe_ground_where(
            PROBE,
            pose(0.0, 0.0),
            options,
            RaycastFilter2d {
                exclude: Some(entity(2)),
                ..RaycastFilter2d::default()
            },
            |id| id != entity(3),
        )
        .unwrap();
    assert!(ground.hit.is_none());
    let ground = world
        .probe_ground(
            PROBE,
            pose(0.0, 0.0),
            options,
            RaycastFilter2d {
                include_sensors: true,
                ..RaycastFilter2d::default()
            },
        )
        .unwrap();
    assert_eq!(ground.hit.unwrap().entity, entity(1));
    let ground = world
        .probe_ground(
            PROBE,
            pose(0.0, 0.0),
            options,
            RaycastFilter2d {
                mask: 0,
                ..RaycastFilter2d::default()
            },
        )
        .unwrap();
    assert!(ground.hit.is_none());
    world.move_to(entity(2), pose(0.0, -4.0)).unwrap();
    let ground = world
        .probe_ground(PROBE, pose(0.0, 0.0), options, RaycastFilter2d::default())
        .unwrap();
    assert_eq!(ground.hit.unwrap().entity, entity(3));
    world.remove(entity(3));
    let ground = world
        .probe_ground(PROBE, pose(0.0, 0.0), options, RaycastFilter2d::default())
        .unwrap();
    assert_eq!(ground.hit.unwrap().entity, entity(2));
    near(ground.hit.unwrap().distance, 2.99);
}

#[test]
fn exact_support_ties_prefer_the_smaller_entity() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    for id in [9, 2] {
        insert(
            &mut world,
            id,
            pose(0.0, -1.0),
            &[Collider2d::rectangle([10.0, 0.5])],
        );
    }
    let ground = world
        .probe_ground(
            PROBE,
            pose(0.0, 0.0),
            GroundOptions2d::default(),
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert_eq!(ground.hit.unwrap().entity, entity(2));
}

#[test]
fn invalid_options_and_overflowing_probe_destinations_fail_before_queries() {
    let world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    for up in [[0.0, 0.0], [0.0, 2.0], [f32::MAX, f32::MAX]] {
        assert_eq!(
            world.probe_ground(
                PROBE,
                pose(0.0, 0.0),
                GroundOptions2d {
                    up,
                    ..GroundOptions2d::default()
                },
                RaycastFilter2d::default()
            ),
            Err(PhysicsError::InvalidGroundUp)
        );
    }
    for angle in [-1.0, f32::NAN, std::f32::consts::PI] {
        assert!(
            world
                .probe_ground(
                    PROBE,
                    pose(0.0, 0.0),
                    GroundOptions2d {
                        max_slope_angle: angle,
                        ..GroundOptions2d::default()
                    },
                    RaycastFilter2d::default()
                )
                .is_err()
        );
    }
    for skin in [0.0, -1.0, f32::INFINITY] {
        assert!(
            world
                .probe_ground(
                    PROBE,
                    pose(0.0, 0.0),
                    GroundOptions2d {
                        skin,
                        ..GroundOptions2d::default()
                    },
                    RaycastFilter2d::default()
                )
                .is_err()
        );
    }
    for distance in [-1.0, f32::NAN, f32::INFINITY] {
        assert!(
            world
                .probe_ground(
                    PROBE,
                    pose(0.0, 0.0),
                    GroundOptions2d {
                        max_distance: distance,
                        ..GroundOptions2d::default()
                    },
                    RaycastFilter2d::default()
                )
                .is_err()
        );
    }
    assert!(
        world
            .probe_ground(
                PROBE,
                pose(0.0, -f32::MAX),
                GroundOptions2d {
                    max_distance: f32::MAX,
                    ..GroundOptions2d::default()
                },
                RaycastFilter2d::default()
            )
            .is_err()
    );
}

#[test]
fn zero_travel_support_allows_only_skin_relative_rounding() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(0.0, -1.0),
        &[Collider2d::rectangle([10.0, 0.5])],
    );
    for skin in [0.01, 0.1] {
        for (gap_factor, walkable) in [(1.005, true), (1.02, false)] {
            let ground = world
                .probe_ground(
                    PROBE,
                    pose(0.0, skin * gap_factor),
                    GroundOptions2d {
                        skin,
                        max_distance: 0.0,
                        ..GroundOptions2d::default()
                    },
                    RaycastFilter2d::default(),
                )
                .unwrap();
            assert_eq!(ground.walkable, walkable);
            if walkable {
                near(ground.hit.unwrap().distance, 0.0);
            }
        }
    }
}
