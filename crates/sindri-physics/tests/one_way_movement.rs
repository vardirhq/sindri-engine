//! Controller policy composes across slide, support, step and carry phases.
#[path = "movement/support.rs"]
mod support;
use sindri_physics::{
    Collider2d, ColliderShape2d, GroundOptions2d, GroundedSlideMotion2d, GroundedSlideOptions2d,
    OneWay2d, PhysicsPose2d, PhysicsWorld2d, PlatformSupport2d, RaycastFilter2d, SlideOptions2d,
};
use support::{entity, insert, near, pose};

const PROBE: ColliderShape2d = ColliderShape2d::Circle { radius: 0.5 };

fn plank() -> PhysicsWorld2d {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(0.0, 0.0),
        &[Collider2d::rectangle([2.0, 0.1])],
    );
    world
        .set_one_way(entity(1), Some(OneWay2d::default()))
        .unwrap();
    world
}

fn movement(
    world: &PhysicsWorld2d,
    at: PhysicsPose2d,
    wanted: [f32; 2],
    options: GroundedSlideOptions2d,
) -> GroundedSlideMotion2d {
    world
        .move_and_slide_grounded(PROBE, at, wanted, options, RaycastFilter2d::default())
        .unwrap()
}

#[test]
fn ascent_passes_and_descent_lands_for_every_probe_shape() {
    let world = plank();
    for shape in [
        PROBE,
        ColliderShape2d::Box {
            half_extents: [0.4, 0.5],
        },
        ColliderShape2d::Capsule {
            half_height: 0.2,
            radius: 0.3,
        },
    ] {
        let rise = world
            .move_and_slide_grounded(
                shape,
                pose(0.0, -1.0),
                [0.0, 2.0],
                GroundedSlideOptions2d::default(),
                RaycastFilter2d::default(),
            )
            .unwrap();
        near(rise.translation[1], 2.0);
        assert!(!rise.grounded && rise.slide.collisions.is_empty());
        let fall = world
            .move_and_slide_grounded(
                shape,
                pose(0.0, 1.0),
                [0.0, -2.0],
                GroundedSlideOptions2d::default(),
                RaycastFilter2d::default(),
            )
            .unwrap();
        near(fall.translation[1], -0.39);
        assert!(fall.grounded && !fall.slide.started_penetrating, "{fall:?}");
        assert_eq!(fall.ground.hit.unwrap().entity, entity(1));
    }
}

#[test]
fn underside_and_deep_overlap_never_grant_support_or_block_escape() {
    let world = plank();
    for at in [pose(0.0, -0.61), pose(0.0, 0.0), pose(0.0, 0.4)] {
        let result = movement(
            &world,
            at,
            [0.0, -1.0],
            GroundedSlideOptions2d {
                snap_distance: 1.0,
                ..GroundedSlideOptions2d::default()
            },
        );
        near(result.translation[1], -1.0);
        assert!(
            !result.grounded && !result.slide.started_penetrating && result.ground.hit.is_none()
        );
    }
    // Front-side shallow penetration is still an explicit obstruction.
    let result = movement(
        &world,
        pose(0.0, 0.59),
        [0.0, -0.1],
        GroundedSlideOptions2d::default(),
    );
    assert!(result.slide.started_penetrating && !result.grounded);
}

#[test]
fn sides_do_not_block_even_with_a_wide_support_cone() {
    let mut world = plank();
    world
        .set_one_way(
            entity(1),
            Some(OneWay2d {
                angle: std::f32::consts::FRAC_PI_2,
                ..OneWay2d::default()
            }),
        )
        .unwrap();
    let result = movement(
        &world,
        pose(-3.0, 0.0),
        [6.0, 0.0],
        GroundedSlideOptions2d::default(),
    );
    near(result.translation[0], 6.0);
    assert!(result.slide.collisions.is_empty());
}

#[test]
fn snap_obeys_support_policy_and_drop_only_ignores_one_way_solids() {
    let mut world = plank();
    insert(
        &mut world,
        2,
        pose(0.0, -2.0),
        &[Collider2d::rectangle([4.0, 0.1])],
    );
    let options = GroundedSlideOptions2d {
        snap_distance: 0.5,
        ..GroundedSlideOptions2d::default()
    };
    let snap = movement(&world, pose(0.0, 0.9), [0.0; 2], options);
    near(snap.snap_translation[1], -0.29);
    assert_eq!(snap.ground.hit.unwrap().entity, entity(1));
    let dropping = GroundedSlideOptions2d {
        drop_through: true,
        ..options
    };
    let no_snap = movement(&world, pose(0.0, 0.61), [0.0; 2], dropping);
    assert!(!no_snap.grounded && no_snap.ground.hit.is_none());
    let fall = movement(&world, pose(0.0, 0.61), [0.0, -4.0], dropping);
    near(fall.translation[1], -2.0);
    assert!(fall.grounded);
    assert_eq!(fall.ground.hit.unwrap().entity, entity(2));
    // Cancelling/expiring the host's request restores policy immediately.
    let again = movement(&world, pose(0.0, 1.0), [0.0, -1.0], options);
    assert_eq!(again.ground.hit.unwrap().entity, entity(1));
}

#[test]
fn ordinary_queries_still_see_both_sides() {
    let world = plank();
    let slide = world
        .move_and_slide(
            PROBE,
            pose(0.0, -1.0),
            [0.0, 2.0],
            SlideOptions2d::default(),
            RaycastFilter2d::default(),
        )
        .unwrap();
    near(slide.translation[1], 0.39);
    let ground = world
        .probe_ground(
            PROBE,
            pose(0.0, 0.0),
            GroundOptions2d::default(),
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(ground.started_penetrating);
    assert_eq!(
        world
            .overlap(PROBE, pose(0.0, 0.0), RaycastFilter2d::default())
            .unwrap(),
        vec![entity(1)]
    );
}

#[test]
fn piece_and_body_rotations_transform_the_support_normal() {
    for piece_rotation in [false, true] {
        let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
        let mut collider = Collider2d::rectangle([2.0, 0.1]);
        let mut at = pose(0.0, 0.0);
        if piece_rotation {
            collider.rotation = -std::f32::consts::FRAC_PI_2;
        } else {
            at.rotation = -std::f32::consts::FRAC_PI_2;
        }
        insert(&mut world, 1, at, &[collider]);
        world
            .set_one_way(
                entity(1),
                Some(OneWay2d {
                    normal: [0.0, 4.0],
                    ..OneWay2d::default()
                }),
            )
            .unwrap();
        let options = GroundedSlideOptions2d {
            up: [1.0, 0.0],
            ..GroundedSlideOptions2d::default()
        };
        let pass = movement(&world, pose(-1.0, 0.0), [2.0, 0.0], options);
        near(pass.translation[0], 2.0);
        let land = movement(&world, pose(1.0, 0.0), [-2.0, 0.0], options);
        near(land.translation[0], -0.39);
        assert!(land.grounded, "{land:?}");
    }
}

#[test]
fn support_cone_rejects_a_top_face_pointing_outside_it() {
    let mut world = plank();
    world
        .set_one_way(
            entity(1),
            Some(OneWay2d {
                normal: [0.5, 1.0],
                angle: 0.1,
            }),
        )
        .unwrap();
    let result = movement(
        &world,
        pose(0.0, 3.0),
        [0.0, -4.0],
        GroundedSlideOptions2d::default(),
    );
    near(result.translation[1], -4.0);
    assert!(!result.grounded);
}

#[test]
fn dropping_cancels_one_way_carry_but_preserves_ordinary_carry() {
    let mut world = plank();
    world.move_to(entity(1), pose(0.4, 0.0)).unwrap();
    let options = GroundedSlideOptions2d {
        platform_support: Some(PlatformSupport2d {
            entity: entity(1),
            previous_pose: pose(0.0, 0.0),
        }),
        ..GroundedSlideOptions2d::default()
    };
    let carried = movement(&world, pose(0.0, 0.61), [0.0; 2], options);
    near(carried.platform.unwrap().motion.translation[0], 0.4);
    let dropping = GroundedSlideOptions2d {
        drop_through: true,
        ..options
    };
    assert!(
        movement(&world, pose(0.0, 0.61), [0.0; 2], dropping)
            .platform
            .is_none()
    );
    world.set_one_way(entity(1), None).unwrap();
    near(
        movement(&world, pose(0.0, 0.61), [0.0; 2], dropping)
            .platform
            .unwrap()
            .motion
            .translation[0],
        0.4,
    );
}

#[test]
fn carry_sweep_uses_one_way_policy_for_other_platforms() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(0.0, -1.0),
        &[Collider2d::rectangle([2.0, 0.5])],
    );
    insert(
        &mut world,
        2,
        pose(0.0, 1.0),
        &[Collider2d::rectangle([2.0, 0.1])],
    );
    world
        .set_one_way(entity(2), Some(OneWay2d::default()))
        .unwrap();
    world.move_to(entity(1), pose(0.0, 1.0)).unwrap();
    let result = movement(
        &world,
        pose(0.0, 0.01),
        [0.0; 2],
        GroundedSlideOptions2d {
            platform_support: Some(PlatformSupport2d {
                entity: entity(1),
                previous_pose: pose(0.0, -1.0),
            }),
            ..GroundedSlideOptions2d::default()
        },
    );
    near(result.platform.unwrap().motion.translation[1], 2.0);
    assert!(result.grounded);
}

#[test]
fn sensors_keep_their_ordinary_geometry_during_drop_requests() {
    let mut world = plank();
    let sensor = Collider2d {
        sensor: true,
        offset: [0.0, -2.0],
        ..Collider2d::rectangle([2.0, 0.1])
    };
    // set_one_way applies only to the solid piece in this compound entity.
    world.remove(entity(1));
    insert(
        &mut world,
        1,
        pose(0.0, 0.0),
        &[Collider2d::rectangle([2.0, 0.1]), sensor],
    );
    world
        .set_one_way(entity(1), Some(OneWay2d::default()))
        .unwrap();
    for include_sensors in [false, true] {
        let result = world
            .move_and_slide_grounded(
                PROBE,
                pose(0.0, 1.0),
                [0.0, -4.0],
                GroundedSlideOptions2d {
                    drop_through: true,
                    ..GroundedSlideOptions2d::default()
                },
                RaycastFilter2d {
                    include_sensors,
                    ..RaycastFilter2d::default()
                },
            )
            .unwrap();
        if include_sensors {
            near(result.translation[1], -2.39);
            assert!(result.grounded);
        } else {
            near(result.translation[1], -4.0);
            assert!(!result.grounded);
        }
    }
}

#[test]
fn step_landing_can_use_one_way_support_unless_dropping() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(0.0, -1.0),
        &[Collider2d::rectangle([4.0, 0.5])],
    );
    insert(
        &mut world,
        2,
        pose(1.5, -0.4),
        &[Collider2d::rectangle([0.5, 0.1])],
    );
    world
        .set_one_way(entity(2), Some(OneWay2d::default()))
        .unwrap();
    // Ordinary riser causes the lift; one-way deck supplies its higher landing.
    insert(
        &mut world,
        3,
        pose(0.8, -0.4),
        &[Collider2d::rectangle([0.1, 0.1])],
    );
    let options = GroundedSlideOptions2d {
        step_height: 0.4,
        ..GroundedSlideOptions2d::default()
    };
    let result = movement(&world, pose(0.0, 0.01), [1.5, 0.0], options);
    near(result.step_translation[1], 0.4);
    near(result.translation[1], 0.2);
    assert_eq!(result.ground.hit.unwrap().entity, entity(2));
    let drop = movement(
        &world,
        pose(0.0, 0.01),
        [1.5, 0.0],
        GroundedSlideOptions2d {
            drop_through: true,
            ..options
        },
    );
    assert_eq!(drop.ground.hit.unwrap().entity, entity(1));
    near(drop.translation[1], 0.0);
}

#[test]
fn extreme_valid_normal_lengths_are_normalized_without_overflow_or_underflow() {
    let mut world = plank();
    for length in [1.0e30, 1.0e-30] {
        world
            .set_one_way(
                entity(1),
                Some(OneWay2d {
                    normal: [0.0, length],
                    ..OneWay2d::default()
                }),
            )
            .unwrap();
        let result = movement(
            &world,
            pose(0.0, 1.0),
            [0.0, -2.0],
            GroundedSlideOptions2d::default(),
        );
        near(result.translation[1], -0.39);
        assert!(result.grounded);
    }
}

#[test]
fn one_way_support_retains_masks_exclusions_predicates_and_entity_ties() {
    let mut world = plank();
    insert(
        &mut world,
        2,
        pose(0.0, 0.0),
        &[Collider2d::rectangle([2.0, 0.1])],
    );
    let mut ignored = Collider2d::rectangle([2.0, 0.1]);
    ignored.layers = sindri_physics::CollisionLayers::new(2, u32::MAX);
    insert(&mut world, 3, pose(0.0, 2.0), &[ignored]);
    world
        .set_one_way(entity(3), Some(OneWay2d::default()))
        .unwrap();
    for exclude in [None, Some(entity(1))] {
        let result = world
            .move_and_slide_grounded_where(
                PROBE,
                pose(0.0, 3.0),
                [0.0, -4.0],
                GroundedSlideOptions2d::default(),
                RaycastFilter2d {
                    mask: 1,
                    exclude,
                    ..RaycastFilter2d::default()
                },
                |_| true,
            )
            .unwrap();
        assert_eq!(
            result.ground.hit.unwrap().entity,
            exclude.map_or(entity(1), |_| entity(2))
        );
        near(result.translation[1], -2.39);
    }
    let inactive = world
        .move_and_slide_grounded_where(
            PROBE,
            pose(0.0, 3.0),
            [0.0, -4.0],
            GroundedSlideOptions2d::default(),
            RaycastFilter2d {
                mask: 1,
                ..RaycastFilter2d::default()
            },
            |id| id != entity(1),
        )
        .unwrap();
    assert_eq!(inactive.ground.hit.unwrap().entity, entity(2));
}

#[test]
fn drop_preserves_ordinary_overlap_and_iteration_outcomes() {
    let mut world = plank();
    insert(
        &mut world,
        2,
        pose(3.0, 0.0),
        &[Collider2d::rectangle([0.1, 4.0])],
    );
    let options = GroundedSlideOptions2d {
        drop_through: true,
        slide: SlideOptions2d {
            max_iterations: 1,
            ..SlideOptions2d::default()
        },
        ..GroundedSlideOptions2d::default()
    };
    let blocked = movement(&world, pose(0.0, 1.0), [4.0, -2.0], options);
    assert!(blocked.slide.iteration_limit_reached);
    assert_eq!(blocked.slide.collisions[0].entity, entity(2));
    let overlap = movement(&world, pose(3.0, 0.0), [1.0, 0.0], options);
    assert!(overlap.slide.started_penetrating && !overlap.grounded);
    near(overlap.translation[0], 0.0);
}
