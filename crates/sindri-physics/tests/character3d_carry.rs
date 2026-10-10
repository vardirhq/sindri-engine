//! Generic carry composition; scene ownership and gameplay are separate.
use sindri_core::EntityId;
use sindri_physics::{
    CharacterOptions3d, Collider3d, GroundedCharacterMotion3d, GroundedCharacterOptions3d,
    PhysicsPose3d, PhysicsWorld3d, PlatformSupport3d, RaycastFilter3d, RigidBody3d, RigidBodyKind,
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
fn platform() -> (PhysicsWorld3d, PlatformSupport3d) {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let previous_pose = at([0.0, -0.5, 0.0]);
    world
        .insert_body(
            id(1),
            RigidBody3d {
                kind: RigidBodyKind::KinematicPosition,
                position: previous_pose.position,
                ..RigidBody3d::default()
            },
            &[Collider3d::cuboid([5.0, 0.5, 5.0])],
        )
        .unwrap();
    (
        world,
        PlatformSupport3d {
            entity: id(1),
            previous_pose,
        },
    )
}
fn query(
    world: &PhysicsWorld3d,
    pose: PhysicsPose3d,
    wanted: [f32; 3],
    support: Option<PlatformSupport3d>,
) -> GroundedCharacterMotion3d {
    world
        .move_character_grounded(
            Collider3d::sphere(0.5).shape,
            pose,
            wanted,
            GroundedCharacterOptions3d {
                platform_support: support,
                ..GroundedCharacterOptions3d::default()
            },
            RaycastFilter3d::default(),
        )
        .unwrap()
}
fn near(actual: [f32; 3], expected: [f32; 3]) {
    for (a, b) in actual.into_iter().zip(expected) {
        assert!((a - b).abs() < 0.02, "{actual:?} != {expected:?}");
    }
}

#[test]
fn solved_translation_carry_is_separate_from_input_and_read_only() {
    let (mut world, support) = platform();
    world
        .set_kinematic_target(id(1), at([2.0, -0.5, 1.0]))
        .unwrap();
    let pose = at([0.0, 0.51, 0.0]);
    near(
        query(&world, pose, [0.0; 3], Some(support)).translation,
        [0.0; 3],
    );
    world.step(Duration::from_millis(10)).unwrap();
    let held = world.pose(id(1)).unwrap();
    let motion = query(&world, pose, [0.0, 0.0, 0.25], Some(support));
    assert!(motion.grounded && motion.ground.walkable);
    assert_eq!(motion.ground.hit.unwrap().entity, id(1));
    let carry = motion.platform.as_ref().unwrap();
    near(carry.requested, [2.0, 0.0, 1.0]);
    near(carry.motion.translation, carry.requested);
    near(motion.movement.translation, [0.0, 0.0, 0.25]);
    near(motion.translation, [2.0, 0.0, 1.25]);
    assert_eq!(world.pose(id(1)).unwrap(), held);
    assert_eq!(
        query(&world.clone(), pose, [0.0, 0.0, 0.25], Some(support)),
        motion
    );
    let endpoint = at([2.0, 0.51, 1.25]);
    near(
        query(
            &world,
            endpoint,
            [0.0; 3],
            Some(PlatformSupport3d {
                entity: id(1),
                previous_pose: held,
            }),
        )
        .translation,
        [0.0; 3],
    );
}

#[test]
fn vertical_platform_motion_is_not_a_jump_and_input_upward_is_not_grounded() {
    for height in [-0.7, -0.3] {
        let (mut world, support) = platform();
        world.move_to(id(1), at([0.0, height, 0.0])).unwrap();
        world.step(Duration::from_millis(10)).unwrap();
        let motion = query(&world, at([0.0, 0.51, 0.0]), [0.0; 3], Some(support));
        near(motion.translation, [0.0, height + 0.5, 0.0]);
        assert!(motion.grounded);
        let jump = query(&world, at([0.0, 0.51, 0.0]), [0.0, 0.4, 0.0], Some(support));
        near(jump.translation, [0.0, height + 0.9, 0.0]);
        assert!(!jump.grounded && !jump.ground.walkable);
    }
}

#[test]
fn rotation_carries_the_probe_origin_without_rotating_it() {
    let (mut world, support) = platform();
    let half = std::f32::consts::FRAC_1_SQRT_2;
    world
        .move_to(
            id(1),
            PhysicsPose3d {
                rotation: [0.0, half, 0.0, half],
                ..support.previous_pose
            },
        )
        .unwrap();
    world.step(Duration::from_millis(10)).unwrap();
    let motion = query(&world, at([2.0, 0.51, 0.0]), [0.0; 3], Some(support));
    near(
        motion.platform.as_ref().unwrap().requested,
        [-2.0, 0.0, -2.0],
    );
    near(motion.translation, [-2.0, 0.0, -2.0]);
    assert!(motion.grounded);
}

#[test]
fn carry_stops_at_walls_and_restores_platform_for_character_movement() {
    let (mut world, support) = platform();
    world
        .insert_static_collider(
            id(2),
            at([2.0, 2.0, 0.0]),
            &[Collider3d::cuboid([0.25, 2.0, 2.0])],
        )
        .unwrap();
    world.move_to(id(1), at([3.0, -0.5, 0.0])).unwrap();
    world.step(Duration::from_millis(10)).unwrap();
    let motion = query(&world, at([0.0, 0.51, 0.0]), [0.0; 3], Some(support));
    let carry = motion.platform.unwrap();
    near(carry.requested, [3.0, 0.0, 0.0]);
    near(carry.motion.translation, [1.24, 0.0, 0.0]);
    assert!(
        carry
            .motion
            .collisions
            .iter()
            .any(|c| c.hit.entity == id(2))
    );
    assert!(motion.grounded && motion.ground.hit.unwrap().entity == id(1));
    near(motion.translation, carry.motion.translation);
    let down = query(
        &world,
        at([0.0, 0.51, 0.0]),
        [0.0, -2.0, 0.0],
        Some(support),
    );
    assert!(
        down.movement
            .collisions
            .iter()
            .any(|c| c.hit.entity == id(1))
    );
    assert!(down.grounded);
}

#[test]
fn missing_filtered_steep_penetrating_and_detached_support_never_carries() {
    let (mut world, support) = platform();
    world.move_to(id(1), at([1.0, -0.5, 0.0])).unwrap();
    for pose in [at([0.0, 0.6, 0.0]), at([0.0, 0.4, 0.0])] {
        assert!(
            query(&world, pose, [0.0; 3], Some(support))
                .platform
                .is_none()
        );
    }
    for filter in [
        RaycastFilter3d {
            exclude: Some(id(1)),
            ..RaycastFilter3d::default()
        },
        RaycastFilter3d {
            mask: 0,
            ..RaycastFilter3d::default()
        },
    ] {
        let motion = world
            .move_character_grounded(
                Collider3d::sphere(0.5).shape,
                at([0.0, 0.51, 0.0]),
                [0.0; 3],
                GroundedCharacterOptions3d {
                    platform_support: Some(support),
                    ..GroundedCharacterOptions3d::default()
                },
                filter,
            )
            .unwrap();
        assert!(motion.platform.is_none());
    }
    let rejected = world
        .move_character_grounded_where(
            Collider3d::sphere(0.5).shape,
            at([0.0, 0.51, 0.0]),
            [0.0; 3],
            GroundedCharacterOptions3d {
                platform_support: Some(support),
                ..GroundedCharacterOptions3d::default()
            },
            RaycastFilter3d::default(),
            |entity| entity != id(1),
        )
        .unwrap();
    assert!(rejected.platform.is_none());
    world.remove(id(1));
    assert!(
        query(&world, at([0.0, 0.51, 0.0]), [0.0; 3], Some(support))
            .platform
            .is_none()
    );
    // Current compound geometry is checked at the previous pose, including sensors.
    world
        .insert_static_collider(
            id(1),
            support.previous_pose,
            &[Collider3d {
                sensor: true,
                ..Collider3d::cuboid([5.0, 0.5, 5.0])
            }],
        )
        .unwrap();
    assert!(
        query(&world, at([0.0, 0.51, 0.0]), [0.0; 3], Some(support))
            .platform
            .is_none()
    );
}

#[test]
fn slope_limits_classify_support_independently_of_rapier_prediction() {
    let (world, _) = platform();
    let motion = query(&world, at([0.0, 0.54, 0.0]), [0.0; 3], None);
    assert!(motion.movement.grounded);
    assert!(!motion.grounded && motion.ground.hit.is_none());
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let angle = std::f32::consts::PI / 6.0;
    let (sin, cos) = (angle * 0.5).sin_cos();
    let previous_pose = PhysicsPose3d {
        rotation: [0.0, 0.0, sin, cos],
        ..at([0.0; 3])
    };
    world
        .insert_static_collider(
            id(1),
            previous_pose,
            &[Collider3d::cuboid([3.0, 0.25, 3.0])],
        )
        .unwrap();
    let position = [0.0, (0.25 + 0.5 + 0.01) / angle.cos(), 0.0];
    let result = world
        .move_character_grounded(
            Collider3d::sphere(0.5).shape,
            at(position),
            [0.0; 3],
            GroundedCharacterOptions3d {
                platform_support: Some(PlatformSupport3d {
                    entity: id(1),
                    previous_pose,
                }),
                movement: CharacterOptions3d {
                    max_slope_angle: 0.1,
                    ..CharacterOptions3d::default()
                },
            },
            RaycastFilter3d::default(),
        )
        .unwrap();
    assert!(result.platform.is_none() && !result.grounded);
}

#[test]
fn invalid_support_and_combined_destinations_fail_without_mutation() {
    let (mut world, support) = platform();
    let pose = world.pose(id(1)).unwrap();
    for previous_pose in [
        at([f32::NAN; 3]),
        PhysicsPose3d {
            rotation: [0.0; 4],
            ..support.previous_pose
        },
    ] {
        let bad = PlatformSupport3d {
            previous_pose,
            entity: id(99),
        };
        assert!(
            world
                .move_character_grounded(
                    Collider3d::sphere(0.5).shape,
                    at([0.0, 0.51, 0.0]),
                    [0.0; 3],
                    GroundedCharacterOptions3d {
                        platform_support: Some(bad),
                        ..GroundedCharacterOptions3d::default()
                    },
                    RaycastFilter3d::default()
                )
                .is_err()
        );
    }
    world.remove(id(1));
    world
        .insert_static_collider(
            id(1),
            at([f32::MAX, -0.5, 0.0]),
            &[Collider3d::cuboid([5.0, 0.5, 5.0])],
        )
        .unwrap();
    assert!(
        world
            .move_character_grounded(
                Collider3d::sphere(0.5).shape,
                at([0.0, 0.51, 0.0]),
                [0.0; 3],
                GroundedCharacterOptions3d {
                    platform_support: Some(support),
                    ..GroundedCharacterOptions3d::default()
                },
                RaycastFilter3d::default()
            )
            .is_err()
    );
    world.move_to(id(1), pose).unwrap();
    assert_eq!(world.pose(id(1)).unwrap(), pose);
}

#[test]
fn compound_offset_support_verifies_old_geometry_outside_current_bounds() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let support = PlatformSupport3d {
        entity: id(1),
        previous_pose: at([0.0; 3]),
    };
    world
        .insert_static_collider(
            id(1),
            at([100.0; 3]),
            &[
                Collider3d {
                    sensor: true,
                    ..Collider3d::sphere(50.0)
                },
                Collider3d {
                    offset: [2.0, -0.5, 0.0],
                    ..Collider3d::cuboid([2.0, 0.5, 2.0])
                },
            ],
        )
        .unwrap();
    let motion = query(&world, at([2.0, 0.51, 0.0]), [0.0; 3], Some(support));
    near(motion.platform.as_ref().unwrap().requested, [100.0; 3]);
    near(motion.translation, [100.0; 3]);
    assert!(motion.grounded);
}

#[test]
fn stationary_rotated_support_has_exactly_zero_carry_without_rounding_drift() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let (sin, cos) = (0.37_f32 * 0.5).sin_cos();
    world
        .insert_static_collider(
            id(1),
            PhysicsPose3d {
                rotation: [0.0, sin, 0.0, cos],
                ..at([0.0, -0.5, 0.0])
            },
            &[Collider3d::cuboid([5.0, 0.5, 5.0])],
        )
        .unwrap();
    let support = PlatformSupport3d {
        entity: id(1),
        previous_pose: world.pose(id(1)).unwrap(),
    };
    let motion = query(&world, at([1.7, 0.51, 1.2]), [0.0; 3], Some(support));
    let carry = motion.platform.unwrap();
    assert!(
        carry
            .requested
            .into_iter()
            .all(|axis| axis.abs() < f32::EPSILON)
    );
    assert!(
        carry
            .motion
            .translation
            .into_iter()
            .all(|axis| axis.abs() < f32::EPSILON)
    );
    assert!(carry.motion.collisions.is_empty());
}
