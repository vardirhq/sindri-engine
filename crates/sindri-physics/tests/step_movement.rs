//! Step proposals must retain support, shape clearance and baseline fallback.
#[path = "movement/support.rs"]
mod support;
use sindri_physics::{
    Collider2d, ColliderShape2d, GroundedSlideMotion2d, GroundedSlideOptions2d, PhysicsPose2d,
    PhysicsWorld2d, RaycastFilter2d,
};
use support::{entity, insert, near, pose};

const PROBE: ColliderShape2d = ColliderShape2d::Box {
    half_extents: [0.4, 0.5],
};

fn stairs(height: f32) -> PhysicsWorld2d {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(0.0, -1.0),
        &[Collider2d::rectangle([10.0, 0.5])],
    );
    insert(
        &mut world,
        2,
        pose(1.5, -0.5 + height / 2.0),
        &[Collider2d::rectangle([0.5, height / 2.0])],
    );
    world
}

fn move_probe(
    world: &PhysicsWorld2d,
    at: PhysicsPose2d,
    displacement: [f32; 2],
    height: f32,
) -> GroundedSlideMotion2d {
    world
        .move_and_slide_grounded(
            PROBE,
            at,
            displacement,
            GroundedSlideOptions2d {
                step_height: height,
                ..GroundedSlideOptions2d::default()
            },
            RaycastFilter2d::default(),
        )
        .unwrap()
}

#[test]
fn low_step_requires_opt_in_and_returns_the_selected_path_parts() {
    let world = stairs(0.25);
    let before = world.pose(entity(2)).unwrap();
    let blocked = move_probe(&world, pose(0.0, 0.01), [1.5, 0.0], 0.0);
    assert!(blocked.translation[0] < 0.7);
    near(blocked.step_translation[1], 0.0);
    let result = move_probe(&world, pose(0.0, 0.01), [1.5, 0.0], 0.3);
    assert!(result.grounded && !result.slide.started_penetrating);
    near(result.translation[0], 1.5);
    near(result.translation[1], 0.25);
    near(result.step_translation[1], 0.3);
    near(result.slide.translation[0], 1.5);
    near(result.slide.translation[1], 0.0);
    near(result.snap_translation[1], -0.05);
    assert_eq!(result.ground.hit.unwrap().entity, entity(2));
    for axis in 0..2 {
        near(
            result.translation[axis],
            result.step_translation[axis]
                + result.slide.translation[axis]
                + result.snap_translation[axis],
        );
    }
    assert_eq!(world.pose(entity(2)).unwrap(), before);
    let next = move_probe(
        &world,
        pose(1.5, 0.01 + result.translation[1]),
        [0.0; 2],
        0.3,
    );
    assert!(next.grounded && !next.slide.started_penetrating);
    near(next.step_translation[1], 0.0);
}

#[test]
fn tall_walls_and_disabled_steps_retain_the_exact_baseline() {
    for height in [0.31, 1.0, 4.0] {
        let world = stairs(height);
        let baseline = move_probe(&world, pose(0.0, 0.01), [1.5, 0.0], 0.0);
        let attempted = move_probe(&world, pose(0.0, 0.01), [1.5, 0.0], 0.3);
        assert_eq!(attempted, baseline, "obstacle height {height}");
    }
}

#[test]
fn step_at_the_height_limit_preserves_surface_skin() {
    let world = stairs(0.3);
    let result = move_probe(&world, pose(0.0, 0.01), [1.5, 0.0], 0.3);
    assert!(result.grounded);
    near(result.translation[0], 1.5);
    near(result.translation[1], 0.3);
    assert!(result.snap_translation[1] <= 0.0);
    near(
        result.step_translation[1] + result.slide.translation[1] + result.snap_translation[1],
        0.3,
    );
    assert!(result.step_translation[1] > 0.29);
}

#[test]
fn ceiling_clearance_and_forward_overhangs_reject_the_candidate() {
    for (position, half_extents) in [([0.0, 0.8], [10.0, 0.1]), ([1.5, 0.7], [0.5, 0.1])] {
        let mut world = stairs(0.25);
        insert(
            &mut world,
            3,
            pose(position[0], position[1]),
            &[Collider2d::rectangle(half_extents)],
        );
        let baseline = move_probe(&world, pose(0.0, 0.01), [1.5, 0.0], 0.0);
        let attempted = move_probe(&world, pose(0.0, 0.01), [1.5, 0.0], 0.3);
        assert_eq!(attempted, baseline);
        assert!(!attempted.slide.started_penetrating);
    }
}

#[test]
fn unsupported_ascent_and_penetration_do_not_attempt_steps() {
    let world = stairs(0.25);
    for (at, displacement) in [
        (pose(0.0, 0.06), [1.5, 0.0]),
        (pose(0.0, 0.01), [1.5, 0.01]),
        (pose(0.0, -0.1), [1.5, 0.0]),
    ] {
        let baseline = move_probe(&world, at, displacement, 0.0);
        let attempted = move_probe(&world, at, displacement, 0.3);
        assert_eq!(attempted, baseline);
    }
}

#[test]
fn no_landing_does_not_convert_a_blocked_move_into_an_airborne_step() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(0.0, -1.0),
        &[Collider2d::rectangle([0.75, 0.5])],
    );
    insert(
        &mut world,
        2,
        pose(1.2, -0.375),
        &[Collider2d::rectangle([0.2, 0.125])],
    );
    let baseline = move_probe(&world, pose(0.0, 0.01), [2.0, 0.0], 0.0);
    let attempted = move_probe(&world, pose(0.0, 0.01), [2.0, 0.0], 0.3);
    assert_eq!(attempted, baseline);
}

#[test]
fn arbitrary_up_uses_a_rotated_probe_for_all_step_phases() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(1.0, 0.0),
        &[Collider2d::rectangle([0.5, 10.0])],
    );
    insert(
        &mut world,
        2,
        pose(0.375, 1.5),
        &[Collider2d::rectangle([0.125, 0.5])],
    );
    let result = world
        .move_and_slide_grounded(
            PROBE,
            PhysicsPose2d {
                position: [-0.01, 0.0],
                rotation: std::f32::consts::FRAC_PI_2,
            },
            [0.0, 1.5],
            GroundedSlideOptions2d {
                up: [-1.0, 0.0],
                step_height: 0.3,
                ..GroundedSlideOptions2d::default()
            },
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(result.grounded);
    near(result.translation[0], -0.25);
    near(result.translation[1], 1.5);
    near(result.step_translation[0], -0.3);
    near(result.snap_translation[0], 0.05);
}

#[test]
fn excluded_or_inactive_obstacles_do_not_trigger_a_step() {
    let world = stairs(0.25);
    let options = GroundedSlideOptions2d {
        step_height: 0.3,
        snap_distance: 0.01,
        ..GroundedSlideOptions2d::default()
    };
    let excluded = world
        .move_and_slide_grounded(
            PROBE,
            pose(0.0, 0.01),
            [1.5, 0.0],
            options,
            RaycastFilter2d {
                exclude: Some(entity(2)),
                ..RaycastFilter2d::default()
            },
        )
        .unwrap();
    let inactive = world
        .move_and_slide_grounded_where(
            PROBE,
            pose(0.0, 0.01),
            [1.5, 0.0],
            options,
            RaycastFilter2d::default(),
            |id| id != entity(2),
        )
        .unwrap();
    let baseline = world
        .move_and_slide_grounded(
            PROBE,
            pose(0.0, 0.01),
            [1.5, 0.0],
            GroundedSlideOptions2d {
                step_height: 0.0,
                ..options
            },
            RaycastFilter2d {
                exclude: Some(entity(2)),
                ..RaycastFilter2d::default()
            },
        )
        .unwrap();
    for result in [excluded, inactive] {
        assert_eq!(result, baseline);
        assert!(result.translation[0] > 1.49);
        near(result.translation[1], 0.0);
        near(result.step_translation[1], 0.0);
        assert!(result.grounded, "{result:?}");
    }
}

#[test]
fn unobstructed_motion_and_zero_motion_do_not_add_a_step_lift() {
    let world = stairs(0.25);
    for displacement in [[0.0; 2], [0.2, 0.0]] {
        let baseline = move_probe(&world, pose(0.0, 0.01), displacement, 0.0);
        let attempted = move_probe(&world, pose(0.0, 0.01), displacement, 0.3);
        assert_eq!(attempted, baseline);
    }
}

#[test]
fn invalid_step_heights_fail_even_if_no_obstacle_is_present() {
    let world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    for height in [-0.1, f32::NAN, f32::INFINITY] {
        let result = world.move_and_slide_grounded(
            PROBE,
            pose(0.0, 0.0),
            [0.0; 2],
            GroundedSlideOptions2d {
                step_height: height,
                ..GroundedSlideOptions2d::default()
            },
            RaycastFilter2d::default(),
        );
        assert!(result.is_err());
    }
}

#[test]
fn circle_and_capsule_step_using_their_actual_extents() {
    let world = stairs(0.45);
    for shape in [
        ColliderShape2d::Circle { radius: 0.5 },
        ColliderShape2d::Capsule {
            half_height: 0.2,
            radius: 0.3,
        },
    ] {
        let result = world
            .move_and_slide_grounded(
                shape,
                pose(0.0, 0.01),
                [1.5, 0.0],
                GroundedSlideOptions2d {
                    step_height: 0.5,
                    ..GroundedSlideOptions2d::default()
                },
                RaycastFilter2d::default(),
            )
            .unwrap();
        assert!(
            result.grounded && result.step_translation[1] > 0.49,
            "{result:?}"
        );
        near(result.translation[0], 1.5);
        near(result.translation[1], 0.45);
    }
}

#[test]
fn steep_support_cannot_become_a_step_landing_above_a_floor() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(0.0, -1.0),
        &[Collider2d::rectangle([10.0, 0.5])],
    );
    insert(
        &mut world,
        2,
        PhysicsPose2d {
            position: [1.5, -0.3],
            rotation: 60.0_f32.to_radians(),
        },
        &[Collider2d::rectangle([1.0, 0.05])],
    );
    let shape = ColliderShape2d::Circle { radius: 0.5 };
    let baseline = world
        .move_and_slide_grounded(
            shape,
            pose(0.0, 0.01),
            [1.5, 0.0],
            GroundedSlideOptions2d::default(),
            RaycastFilter2d::default(),
        )
        .unwrap();
    let attempted = world
        .move_and_slide_grounded(
            shape,
            pose(0.0, 0.01),
            [1.5, 0.0],
            GroundedSlideOptions2d {
                step_height: 1.5,
                ..GroundedSlideOptions2d::default()
            },
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert_eq!(attempted, baseline);
    near(attempted.step_translation[1], 0.0);
}
