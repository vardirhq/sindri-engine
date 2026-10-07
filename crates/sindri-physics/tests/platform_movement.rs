//! Carry uses synchronized pose deltas and precedes character movement once.
use std::time::Duration;
#[path = "movement/support.rs"]
mod support;
use sindri_physics::{
    Collider2d, ColliderShape2d, GroundedSlideMotion2d, GroundedSlideOptions2d, PhysicsPose2d,
    PhysicsWorld2d, PlatformSupport2d, RaycastFilter2d, RigidBody2d, RigidBodyKind,
};
use support::{entity, insert, near, pose};

const PROBE: ColliderShape2d = ColliderShape2d::Circle { radius: 0.5 };

fn platform() -> PhysicsWorld2d {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(0.0, -1.0),
        &[Collider2d::rectangle([2.0, 0.5])],
    );
    world
}

fn options(previous_pose: PhysicsPose2d) -> GroundedSlideOptions2d {
    GroundedSlideOptions2d {
        platform_support: Some(PlatformSupport2d {
            entity: entity(1),
            previous_pose,
        }),
        ..GroundedSlideOptions2d::default()
    }
}

fn move_probe(
    world: &PhysicsWorld2d,
    at: PhysicsPose2d,
    wanted: [f32; 2],
    previous: PhysicsPose2d,
) -> GroundedSlideMotion2d {
    world
        .move_and_slide_grounded(
            PROBE,
            at,
            wanted,
            options(previous),
            RaycastFilter2d::default(),
        )
        .unwrap()
}

#[test]
fn synchronized_translation_carries_all_shapes_before_their_own_movement() {
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
        for movement in [[0.4, 0.0], [0.0, 0.2], [0.0, -0.2]] {
            let mut world = platform();
            let previous = world.pose(entity(1)).unwrap();
            let current = pose(movement[0], -1.0 + movement[1]);
            world.move_to(entity(1), current).unwrap();
            let result = world
                .move_and_slide_grounded(
                    shape,
                    pose(0.0, 0.01),
                    [0.1, 0.0],
                    options(previous),
                    RaycastFilter2d::default(),
                )
                .unwrap();
            let carry = result.platform.as_ref().unwrap();
            assert_eq!(carry.entity, entity(1));
            assert_eq!(carry.current_pose, current);
            near(carry.requested_translation[0], movement[0]);
            near(carry.motion.translation[1], movement[1]);
            near(result.translation[0], movement[0] + 0.1);
            near(result.translation[1], movement[1]);
            assert!(
                result.grounded && !result.slide.started_penetrating,
                "{result:?}"
            );
            assert_eq!(world.pose(entity(1)).unwrap(), current);
        }
    }
}

#[test]
fn advancing_snapshot_avoids_reapplying_the_previous_frame_motion() {
    let mut world = platform();
    let previous = world.pose(entity(1)).unwrap();
    world.move_to(entity(1), pose(0.3, -0.9)).unwrap();
    let first = move_probe(&world, pose(0.0, 0.01), [0.0; 2], previous);
    let snapshot = first.platform.unwrap().current_pose;
    let at = pose(first.translation[0], 0.01 + first.translation[1]);
    let stationary = move_probe(&world, at, [0.0; 2], snapshot);
    near(stationary.translation[0], 0.0);
    near(stationary.translation[1], 0.0);
    assert!(stationary.grounded);
    world.move_to(entity(1), pose(0.5, -0.9)).unwrap();
    let next = move_probe(&world, at, [0.0; 2], snapshot);
    near(next.translation[0], 0.2);
    near(next.translation[1], 0.0);
}

#[test]
fn rotation_moves_the_previous_probe_origin_about_the_support_body() {
    let mut world = platform();
    let previous = world.pose(entity(1)).unwrap();
    let angle = 10.0_f32.to_radians();
    world
        .move_to(
            entity(1),
            PhysicsPose2d {
                position: previous.position,
                rotation: angle,
            },
        )
        .unwrap();
    let result = move_probe(&world, pose(1.0, 0.01), [0.0; 2], previous);
    let expected = [
        angle.cos() - 1.01 * angle.sin() - 1.0,
        angle.sin() + 1.01 * angle.cos() - 1.01,
    ];
    let carry = result.platform.as_ref().unwrap();
    near(carry.requested_translation[0], expected[0]);
    near(carry.requested_translation[1], expected[1]);
    near(result.translation[0], expected[0]);
    near(result.translation[1], expected[1]);
    assert!(result.grounded && !result.slide.started_penetrating);
}

#[test]
fn wall_limits_carry_without_adding_platform_velocity_or_double_motion() {
    let mut world = platform();
    let previous = world.pose(entity(1)).unwrap();
    insert(
        &mut world,
        2,
        pose(0.8, 0.0),
        &[Collider2d::rectangle([0.1, 2.0])],
    );
    world.move_to(entity(1), pose(0.5, -1.0)).unwrap();
    let result = move_probe(&world, pose(0.0, 0.01), [0.0; 2], previous);
    let carry = result.platform.as_ref().unwrap();
    near(carry.requested_translation[0], 0.5);
    near(carry.motion.translation[0], 0.19);
    assert_eq!(carry.motion.collisions[0].entity, entity(2));
    near(result.translation[0], 0.19);
    assert!(result.grounded);
    // A blocked frame still advances the support snapshot.
    world.move_to(entity(1), pose(1.0, -1.0)).unwrap();
    let blocked = move_probe(&world, pose(0.19, 0.01), [0.0; 2], carry.current_pose);
    near(blocked.translation[0], 0.0);
    near(blocked.platform.unwrap().requested_translation[0], 0.5);
}

#[test]
fn ceiling_crush_reports_penetration_instead_of_pushing_through_geometry() {
    let mut world = platform();
    let previous = world.pose(entity(1)).unwrap();
    insert(
        &mut world,
        2,
        pose(0.0, 0.7),
        &[Collider2d::rectangle([2.0, 0.1])],
    );
    world.move_to(entity(1), pose(0.0, -0.8)).unwrap();
    let result = move_probe(&world, pose(0.0, 0.01), [0.0; 2], previous);
    let carry = result.platform.as_ref().unwrap();
    assert_eq!(carry.motion.collisions[0].entity, entity(2));
    near(carry.motion.translation[1], 0.08);
    assert!(result.slide.started_penetrating && result.ground.started_penetrating);
    assert!(!result.grounded);
    assert!(result.translation[1] < 0.1);
}

#[test]
fn jumps_and_ledge_departure_use_carry_then_lose_support() {
    let mut world = platform();
    let previous = world.pose(entity(1)).unwrap();
    world.move_to(entity(1), pose(0.2, -0.9)).unwrap();
    let jump = move_probe(&world, pose(0.0, 0.01), [0.0, 0.2], previous);
    near(jump.translation[0], 0.2);
    near(jump.translation[1], 0.3);
    assert!(!jump.grounded && jump.platform.is_some());
    let leaving = move_probe(&world, pose(0.0, 0.01), [3.0, 0.0], previous);
    near(leaving.translation[0], 3.2);
    assert!(!leaving.grounded && leaving.ground.hit.is_none());
}

#[test]
fn removed_excluded_masked_and_inactive_support_do_not_carry() {
    let mut world = platform();
    let previous = world.pose(entity(1)).unwrap();
    world.move_to(entity(1), pose(0.3, -1.0)).unwrap();
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
            .move_and_slide_grounded(PROBE, pose(0.0, 0.01), [0.0; 2], options(previous), filter)
            .unwrap();
        assert!(result.platform.is_none());
        near(result.translation[0], 0.0);
    }
    let inactive = world
        .move_and_slide_grounded_where(
            PROBE,
            pose(0.0, 0.01),
            [0.0; 2],
            options(previous),
            RaycastFilter2d::default(),
            |_| false,
        )
        .unwrap();
    assert!(inactive.platform.is_none());
    world.remove(entity(1));
    let removed = move_probe(&world, pose(0.0, 0.01), [0.0; 2], previous);
    assert!(removed.platform.is_none() && !removed.grounded);
}

#[test]
fn stale_snapshot_does_not_carry_unsupported_or_penetrating_characters() {
    let mut world = platform();
    let previous = world.pose(entity(1)).unwrap();
    world.move_to(entity(1), pose(0.3, -1.0)).unwrap();
    for at in [pose(0.0, 0.2), pose(0.0, -0.1)] {
        let result = move_probe(&world, at, [0.0; 2], previous);
        assert!(result.platform.is_none());
        near(result.translation[0], 0.0);
    }
}

#[test]
fn compound_offsets_and_local_rotations_are_reconstructed_at_previous_pose() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(-3.0, -1.0),
        &[
            Collider2d {
                offset: [3.0, 0.0],
                rotation: std::f32::consts::FRAC_PI_2,
                ..Collider2d::rectangle([0.5, 2.0])
            },
            Collider2d {
                offset: [-3.0, 0.0],
                ..Collider2d::circle(0.2)
            },
        ],
    );
    let previous = world.pose(entity(1)).unwrap();
    world.move_to(entity(1), pose(-2.7, -1.0)).unwrap();
    let result = move_probe(&world, pose(0.0, 0.01), [0.0; 2], previous);
    assert!(result.platform.is_some() && result.grounded);
    near(result.translation[0], 0.3);
}

#[test]
fn carry_and_step_parts_sum_to_one_total_proposal() {
    let mut world = platform();
    let previous = world.pose(entity(1)).unwrap();
    insert(
        &mut world,
        2,
        pose(1.5, -0.375),
        &[Collider2d::rectangle([0.5, 0.125])],
    );
    world.move_to(entity(1), pose(0.2, -1.0)).unwrap();
    let result = world
        .move_and_slide_grounded(
            ColliderShape2d::Box {
                half_extents: [0.4, 0.5],
            },
            pose(0.0, 0.01),
            [1.3, 0.0],
            GroundedSlideOptions2d {
                step_height: 0.3,
                ..options(previous)
            },
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(result.grounded && result.step_translation[1] > 0.29);
    let carry = result.platform.as_ref().unwrap();
    for axis in 0..2 {
        near(
            result.translation[axis],
            carry.motion.translation[axis]
                + result.step_translation[axis]
                + result.slide.translation[axis]
                + result.snap_translation[axis],
        );
    }
    near(result.translation[0], 1.5);
    near(result.translation[1], 0.25);
}

#[test]
fn invalid_previous_pose_and_input_fail_before_any_carry_query() {
    let world = platform();
    for previous in [
        pose(f32::NAN, -1.0),
        PhysicsPose2d {
            position: [0.0, -1.0],
            rotation: f32::NAN,
        },
    ] {
        let result = world.move_and_slide_grounded(
            PROBE,
            pose(0.0, 0.01),
            [0.0; 2],
            options(previous),
            RaycastFilter2d::default(),
        );
        assert!(result.is_err());
    }
    let result = world.move_and_slide_grounded(
        PROBE,
        pose(0.0, 0.01),
        [f32::MAX, f32::MAX],
        options(pose(0.0, -1.0)),
        RaycastFilter2d::default(),
    );
    assert!(result.is_err());
}

#[test]
fn kinematic_targets_and_velocity_carry_only_after_simulation_updates_the_pose() {
    for kind in [
        RigidBodyKind::KinematicPosition,
        RigidBodyKind::KinematicVelocity,
    ] {
        let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
        world
            .insert_body(
                entity(1),
                RigidBody2d {
                    kind,
                    pose: pose(0.0, -1.0),
                    ..RigidBody2d::default()
                },
                &[Collider2d::rectangle([2.0, 0.5])],
            )
            .unwrap();
        let previous = world.pose(entity(1)).unwrap();
        if kind == RigidBodyKind::KinematicPosition {
            world
                .set_kinematic_target(entity(1), pose(0.2, -1.0))
                .unwrap();
        } else {
            world.set_linear_velocity(entity(1), [2.0, 0.0]).unwrap();
        }
        let pending = move_probe(&world, pose(0.0, 0.01), [0.0; 2], previous);
        near(pending.translation[0], 0.0);
        world.step(Duration::from_millis(100)).unwrap();
        let moved = move_probe(&world, pose(0.0, 0.01), [0.05, 0.0], previous);
        near(moved.platform.unwrap().motion.translation[0], 0.2);
        near(moved.translation[0], 0.25);
        assert!(moved.grounded);
    }
}

#[test]
fn sensor_support_obeys_opt_in_for_verification_and_final_grounding() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(0.0, -1.0),
        &[Collider2d {
            sensor: true,
            ..Collider2d::rectangle([2.0, 0.5])
        }],
    );
    let previous = world.pose(entity(1)).unwrap();
    world.move_to(entity(1), pose(0.2, -1.0)).unwrap();
    let excluded = move_probe(&world, pose(0.0, 0.01), [0.0; 2], previous);
    assert!(excluded.platform.is_none() && !excluded.grounded);
    let included = world
        .move_and_slide_grounded(
            PROBE,
            pose(0.0, 0.01),
            [0.0; 2],
            options(previous),
            RaycastFilter2d {
                include_sensors: true,
                ..RaycastFilter2d::default()
            },
        )
        .unwrap();
    near(included.translation[0], 0.2);
    assert!(included.platform.is_some() && included.grounded);
}

#[test]
fn prior_steep_support_is_rejected_and_new_steep_orientation_loses_grounding() {
    let mut world = platform();
    let flat = world.pose(entity(1)).unwrap();
    let steep = PhysicsPose2d {
        position: [0.0, -1.0],
        rotation: 60.0_f32.to_radians(),
    };
    world.move_to(entity(1), steep).unwrap();
    let tilted = move_probe(&world, pose(0.0, 0.01), [0.0; 2], flat);
    assert!(tilted.platform.is_some() && !tilted.grounded);
    let at = pose(tilted.translation[0], 0.01 + tilted.translation[1]);
    world
        .move_to(
            entity(1),
            PhysicsPose2d {
                position: [0.2, -1.0],
                ..steep
            },
        )
        .unwrap();
    let rejected = move_probe(&world, at, [0.0; 2], steep);
    assert!(rejected.platform.is_none());
}
