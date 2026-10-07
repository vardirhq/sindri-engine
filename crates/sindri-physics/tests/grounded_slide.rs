//! Support is determined after motion; optional snap cannot undo an ascent.
#[path = "movement/support.rs"]
mod support;
use sindri_physics::{
    Collider2d, ColliderShape2d, GroundOptions2d, GroundedSlideMotion2d, GroundedSlideOptions2d,
    PhysicsError, PhysicsPose2d, PhysicsWorld2d, RaycastFilter2d,
};
use support::{entity, insert, near, pose};

const PROBE: ColliderShape2d = ColliderShape2d::Circle { radius: 0.5 };

fn floor() -> PhysicsWorld2d {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(0.0, -1.0),
        &[Collider2d::rectangle([2.0, 0.5])],
    );
    world
}

fn move_probe(
    world: &PhysicsWorld2d,
    at: PhysicsPose2d,
    displacement: [f32; 2],
    snap_distance: f32,
) -> GroundedSlideMotion2d {
    world
        .move_and_slide_grounded(
            PROBE,
            at,
            displacement,
            GroundedSlideOptions2d {
                snap_distance,
                ..GroundedSlideOptions2d::default()
            },
            RaycastFilter2d::default(),
        )
        .unwrap()
}

#[test]
fn support_is_measured_after_movement_and_snap_defaults_off() {
    let world = floor();
    let nearby = move_probe(&world, pose(0.0, 0.06), [0.0; 2], 0.0);
    assert!(!nearby.grounded && nearby.ground.hit.is_none());
    near(nearby.translation[1], 0.0);
    for height in [0.0, 0.005, 0.01] {
        let resting = move_probe(&world, pose(0.0, height), [0.2, 0.0], 0.0);
        assert!(resting.grounded);
        near(resting.translation[0], 0.2);
        near(resting.snap_translation[1], 0.0);
    }
    let leaving = move_probe(&world, pose(0.0, 0.01), [3.0, 0.0], 0.1);
    assert!(!leaving.grounded && leaving.ground.hit.is_none());
    near(leaving.translation[0], 3.0);
}

#[test]
fn snapping_reports_separate_translation_and_preserves_the_world() {
    let world = floor();
    let before = world.pose(entity(1)).unwrap();
    let result = move_probe(&world, pose(0.0, 0.06), [0.2, 0.0], 0.1);
    assert!(result.grounded && result.slide.collisions.is_empty());
    near(result.slide.translation[1], 0.0);
    near(result.snap_translation[1], -0.05);
    near(result.translation[1], -0.05);
    near(result.ground.hit.unwrap().distance, 0.05);
    assert_eq!(world.pose(entity(1)).unwrap(), before);
    let endpoint = pose(0.2, 0.06 + result.translation[1]);
    let support = move_probe(&world, endpoint, [0.0; 2], 0.0);
    assert!(support.grounded && !support.slide.started_penetrating);
    let too_far = move_probe(&world, pose(0.0, 0.3), [0.0; 2], 0.1);
    assert!(!too_far.grounded);
    near(too_far.snap_translation[1], 0.0);
}

#[test]
fn ascending_requests_never_snap_even_when_blocked_by_a_ceiling() {
    let mut world = floor();
    let jump = move_probe(&world, pose(0.0, 0.01), [0.0, 0.04], 0.5);
    assert!(!jump.grounded);
    near(jump.translation[1], 0.04);
    near(jump.snap_translation[1], 0.0);
    insert(
        &mut world,
        2,
        pose(0.0, 1.0),
        &[Collider2d::rectangle([2.0, 0.5])],
    );
    let blocked = move_probe(&world, pose(0.0, 0.0), [0.0, 0.2], 0.5);
    assert!(!blocked.grounded && blocked.ground.walkable);
    assert!(!blocked.slide.collisions.is_empty());
    near(blocked.translation[1], 0.0);
    near(blocked.snap_translation[1], 0.0);
}

#[test]
fn landing_and_repeated_tangent_movement_keep_support() {
    let world = floor();
    let landing = move_probe(&world, pose(0.0, 1.0), [0.2, -2.0], 0.0);
    assert!(landing.grounded && !landing.slide.collisions.is_empty());
    near(landing.translation[1], -0.99);
    let mut at = pose(landing.translation[0], 1.0 + landing.translation[1]);
    for _ in 0..10 {
        let next = move_probe(&world, at, [0.1, -0.02], 0.1);
        assert!(next.grounded && !next.slide.started_penetrating);
        at.position[0] += next.translation[0];
        at.position[1] += next.translation[1];
        near(at.position[1], 0.01);
    }
}

#[test]
fn descending_walkable_slope_can_snap_but_steep_support_cannot() {
    for (angle, walkable) in [(30.0_f32, true), (60.0_f32, false)] {
        let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
        let rotation = angle.to_radians();
        insert(
            &mut world,
            1,
            PhysicsPose2d {
                position: [0.0, 0.0],
                rotation,
            },
            &[Collider2d::rectangle([10.0, 0.5])],
        );
        let height = 1.01 / rotation.cos();
        let result = move_probe(&world, pose(0.0, height), [-0.1, 0.0], 0.5);
        assert_eq!(result.grounded, walkable);
        assert_eq!(result.ground.walkable, walkable);
        assert!(result.ground.hit.is_some());
        near(result.translation[0], -0.1);
        if walkable {
            assert!(result.snap_translation[1] < -0.04);
            let supported = world
                .probe_ground(
                    PROBE,
                    pose(-0.1, height + result.translation[1]),
                    GroundOptions2d {
                        max_distance: 0.0,
                        ..GroundOptions2d::default()
                    },
                    RaycastFilter2d::default(),
                )
                .unwrap();
            assert!(
                supported.walkable && !supported.started_penetrating,
                "{result:?} -> {supported:?}"
            );
        } else {
            near(result.snap_translation[1], 0.0);
        }
    }
}

#[test]
fn initial_penetration_blocks_both_motion_and_snapping() {
    let world = floor();
    let result = move_probe(&world, pose(0.0, -0.2), [1.0, 0.0], 0.5);
    assert!(result.slide.started_penetrating && result.ground.started_penetrating);
    assert!(!result.grounded);
    near(result.translation[0], 0.0);
    near(result.translation[1], 0.0);
    near(result.slide.remaining[0], 1.0);
}

#[test]
fn snap_uses_arbitrary_up_and_actual_rotated_shape() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(-1.0, 0.0),
        &[Collider2d::rectangle([0.5, 10.0])],
    );
    let options = GroundedSlideOptions2d {
        up: [1.0, 0.0],
        snap_distance: 0.1,
        ..GroundedSlideOptions2d::default()
    };
    let result = world
        .move_and_slide_grounded(
            ColliderShape2d::Capsule {
                half_height: 0.4,
                radius: 0.3,
            },
            PhysicsPose2d {
                position: [0.26, 0.0],
                rotation: std::f32::consts::FRAC_PI_2,
            },
            [0.0, 0.2],
            options,
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(result.grounded);
    near(result.snap_translation[0], -0.05);
    near(result.translation[1], 0.2);
    let jump = world
        .move_and_slide_grounded(
            PROBE,
            pose(0.01, 0.0),
            [0.02, 0.0],
            options,
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(!jump.grounded);
    near(jump.snap_translation[0], 0.0);
}

#[test]
fn movement_and_support_share_filters_predicates_and_current_poses() {
    let mut world = floor();
    let options = GroundedSlideOptions2d {
        snap_distance: 0.1,
        ..GroundedSlideOptions2d::default()
    };
    for filter in [
        RaycastFilter2d {
            exclude: Some(entity(1)),
            ..RaycastFilter2d::default()
        },
        RaycastFilter2d {
            mask: 0,
            ..RaycastFilter2d::default()
        },
    ] {
        let result = world
            .move_and_slide_grounded(PROBE, pose(0.0, 0.06), [0.0, -1.0], options, filter)
            .unwrap();
        assert!(!result.grounded && result.slide.collisions.is_empty());
        near(result.translation[1], -1.0);
    }
    let inactive = world
        .move_and_slide_grounded_where(
            PROBE,
            pose(0.0, 0.06),
            [0.0, -1.0],
            options,
            RaycastFilter2d::default(),
            |_| false,
        )
        .unwrap();
    assert!(!inactive.grounded && inactive.slide.collisions.is_empty());
    world.move_to(entity(1), pose(0.0, -2.0)).unwrap();
    assert!(!move_probe(&world, pose(0.0, 0.06), [0.0; 2], 0.1).grounded);
    world.remove(entity(1));
    assert!(!move_probe(&world, pose(0.0, 0.06), [0.0; 2], 2.0).grounded);
}

#[test]
fn invalid_ground_options_are_rejected_even_when_ascent_disables_snap() {
    let world = floor();
    for options in [
        GroundedSlideOptions2d {
            up: [0.0; 2],
            ..GroundedSlideOptions2d::default()
        },
        GroundedSlideOptions2d {
            snap_distance: -0.1,
            ..GroundedSlideOptions2d::default()
        },
        GroundedSlideOptions2d {
            snap_distance: f32::NAN,
            ..GroundedSlideOptions2d::default()
        },
        GroundedSlideOptions2d {
            max_slope_angle: 2.0,
            ..GroundedSlideOptions2d::default()
        },
    ] {
        assert!(
            world
                .move_and_slide_grounded(
                    PROBE,
                    pose(0.0, 1.0),
                    [0.0, 0.1],
                    options,
                    RaycastFilter2d::default()
                )
                .is_err()
        );
    }
    let empty = PhysicsWorld2d::new([0.0; 2]).unwrap();
    let overflow = empty.move_and_slide_grounded(
        PROBE,
        pose(0.0, -f32::MAX),
        [0.0; 2],
        GroundedSlideOptions2d {
            snap_distance: f32::MAX,
            ..GroundedSlideOptions2d::default()
        },
        RaycastFilter2d::default(),
    );
    assert!(matches!(overflow, Err(PhysicsError::NonFinite(_))));
}

#[test]
fn budget_exhaustion_retains_slide_remaining_and_probes_actual_endpoint() {
    let mut world = floor();
    insert(
        &mut world,
        2,
        pose(1.5, 1.0),
        &[Collider2d::rectangle([0.25, 2.0])],
    );
    let mut options = GroundedSlideOptions2d {
        snap_distance: 0.5,
        ..GroundedSlideOptions2d::default()
    };
    options.slide.max_iterations = 1;
    let result = world
        .move_and_slide_grounded(
            PROBE,
            pose(0.0, 1.0),
            [2.0, -2.0],
            options,
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(result.slide.iteration_limit_reached && result.grounded);
    assert!(result.slide.remaining[1] < -0.5);
    near(result.slide.translation[0], 0.74);
    near(result.slide.translation[1], -0.74);
    near(result.snap_translation[1], -0.25);
    near(result.translation[1], -0.99);
}
