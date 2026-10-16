//! Filtering, lifecycle and validation for the read-only 3D controller.

use sindri_core::EntityId;
use sindri_physics::{
    CharacterMotion3d, CharacterOptions3d, Collider3d, ColliderShape3d, CollisionLayers,
    PhysicsError, PhysicsPose3d, PhysicsWorld3d, RaycastFilter3d, RigidBody3d, RigidBodyKind,
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
fn near3(actual: [f32; 3], expected: [f32; 3]) {
    for (a, b) in actual.into_iter().zip(expected) {
        assert!((a - b).abs() < 0.003, "{actual:?} != {expected:?}");
    }
}
fn solid(world: &mut PhysicsWorld3d, index: u32, position: [f32; 3], extents: [f32; 3]) {
    world
        .insert_static_collider(id(index), at(position), &[Collider3d::cuboid(extents)])
        .unwrap();
}
fn move_box(
    world: &PhysicsWorld3d,
    pose: PhysicsPose3d,
    wanted: [f32; 3],
    options: CharacterOptions3d,
) -> CharacterMotion3d {
    world
        .move_character(
            ColliderShape3d::Box {
                half_extents: [0.2, 0.5, 0.2],
            },
            pose,
            wanted,
            options,
            RaycastFilter3d::default(),
        )
        .unwrap()
}

#[test]
fn filtering_uses_memberships_sensors_whole_entities_and_host_predicates() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let piece = Collider3d {
        layers: CollisionLayers {
            memberships: 2,
            filter: 0,
        },
        ..Collider3d::cuboid([0.5, 4.0, 4.0])
    };
    world
        .insert_static_collider(id(1), at([2.0, 0.0, 0.0]), &[piece, piece])
        .unwrap();
    let sensor = Collider3d {
        sensor: true,
        ..Collider3d::cuboid([0.1, 4.0, 4.0])
    };
    world
        .insert_static_collider(id(2), at([0.5, 0.0, 0.0]), &[sensor])
        .unwrap();
    let shape = Collider3d::sphere(0.2).shape;
    for filter in [
        RaycastFilter3d {
            mask: 1,
            ..RaycastFilter3d::default()
        },
        RaycastFilter3d {
            exclude: Some(id(1)),
            ..RaycastFilter3d::default()
        },
    ] {
        let result = world
            .move_character(
                shape,
                at([0.0; 3]),
                [4.0, 0.0, 0.0],
                CharacterOptions3d::default(),
                filter,
            )
            .unwrap();
        near3(result.translation, [4.0, 0.0, 0.0]);
    }
    let result = world
        .move_character_where(
            shape,
            at([0.0; 3]),
            [4.0, 0.0, 0.0],
            CharacterOptions3d::default(),
            RaycastFilter3d::default(),
            |entity| entity != id(1),
        )
        .unwrap();
    near3(result.translation, [4.0, 0.0, 0.0]);
    let blocked = world
        .move_character(
            shape,
            at([0.0; 3]),
            [4.0, 0.0, 0.0],
            CharacterOptions3d::default(),
            RaycastFilter3d::default(),
        )
        .unwrap();
    assert!(blocked.translation[0] < 1.31);
    assert!(
        blocked
            .collisions
            .iter()
            .all(|collision| collision.hit.entity == id(1))
    );
}

#[test]
fn teleports_and_removal_update_controller_geometry_before_the_next_solve() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let piece = Collider3d {
        offset: [1.0, 0.0, 0.0],
        ..Collider3d::cuboid([0.5, 4.0, 4.0])
    };
    world
        .insert_body(
            id(1),
            RigidBody3d {
                position: [1.0, 0.0, 0.0],
                ..RigidBody3d::default()
            },
            &[piece],
        )
        .unwrap();
    let query = |world: &PhysicsWorld3d| {
        move_box(
            world,
            at([0.0; 3]),
            [4.0, 0.0, 0.0],
            CharacterOptions3d::default(),
        )
    };
    near3(query(&world).translation, [1.29, 0.0, 0.0]);
    world.move_to(id(1), at([2.0, 0.0, 0.0])).unwrap();
    near3(query(&world).translation, [2.29, 0.0, 0.0]);
    world.move_to(id(1), at([2.0, 10.0, 0.0])).unwrap();
    near3(query(&world).translation, [4.0, 0.0, 0.0]);
    world.remove(id(1));
    solid(&mut world, 2, [2.0, 0.0, 0.0], [0.5, 4.0, 4.0]);
    assert!(
        query(&world)
            .collisions
            .iter()
            .all(|collision| collision.hit.entity == id(2))
    );
}

#[test]
fn read_only_queries_and_copied_results_preserve_body_motion_and_clone_replay() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_body(
            id(1),
            RigidBody3d {
                position: [2.0, 0.0, 0.0],
                linear_velocity: [0.0, 0.0, 1.0],
                ..RigidBody3d::default()
            },
            &[Collider3d::cuboid([0.5, 4.0, 4.0])],
        )
        .unwrap();
    let before = world.pose(id(1)).unwrap();
    let original = world.clone();
    let mut result = move_box(
        &world,
        at([0.0; 3]),
        [4.0, 0.0, 0.0],
        CharacterOptions3d::default(),
    );
    result.translation = [99.0; 3];
    result.collisions.clear();
    assert_eq!(world.pose(id(1)).unwrap(), before);
    near3(world.linear_velocity(id(1)).unwrap(), [0.0, 0.0, 1.0]);
    near3(
        move_box(
            &world,
            at([0.0; 3]),
            [4.0, 0.0, 0.0],
            CharacterOptions3d::default(),
        )
        .translation,
        [1.29, 0.0, 0.0],
    );
    let mut cloned = original;
    assert_eq!(
        world.step(Duration::from_millis(16)).unwrap(),
        cloned.step(Duration::from_millis(16)).unwrap()
    );
    assert_eq!(world.pose(id(1)).unwrap(), cloned.pose(id(1)).unwrap());
}

#[test]
fn kinematic_velocity_does_not_add_implicit_platform_carry_to_a_displacement_query() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_body(
            id(1),
            RigidBody3d {
                kind: RigidBodyKind::KinematicVelocity,
                position: [0.0, -0.25, 0.0],
                linear_velocity: [10.0, 0.0, 0.0],
                ..RigidBody3d::default()
            },
            &[Collider3d::cuboid([10.0, 0.25, 10.0])],
        )
        .unwrap();
    let result = move_box(
        &world,
        at([0.0, 0.51, 0.0]),
        [0.0, -0.1, 0.0],
        CharacterOptions3d::default(),
    );
    near3(result.translation, [0.0; 3]);
    assert!(result.grounded);
    near3(world.pose(id(1)).unwrap().position, [0.0, -0.25, 0.0]);
}

#[test]
fn stationary_penetration_correction_is_bounded_and_read_only() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    solid(&mut world, 1, [0.0, -0.25, 0.0], [10.0, 0.25, 10.0]);
    let result = move_box(
        &world,
        at([0.0, 0.4, 0.0]),
        [0.0; 3],
        CharacterOptions3d::default(),
    );
    assert!(
        result.translation[1] > 0.1 && result.translation[1] <= 0.25,
        "{result:?}"
    );
    near3(world.pose(id(1)).unwrap().position, [0.0, -0.25, 0.0]);
}

#[test]
fn invalid_inputs_and_extreme_bounds_return_errors_without_mutation() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    solid(&mut world, 1, [2.0, 0.0, 0.0], [0.5, 4.0, 4.0]);
    let shape = Collider3d::sphere(0.2).shape;
    for wanted in [[f32::NAN; 3], [f32::INFINITY; 3], [f32::MAX; 3]] {
        assert!(
            world
                .move_character(
                    shape,
                    at([0.0; 3]),
                    wanted,
                    CharacterOptions3d::default(),
                    RaycastFilter3d::default()
                )
                .is_err()
        );
    }
    for options in [
        CharacterOptions3d {
            up: [0.0; 3],
            ..CharacterOptions3d::default()
        },
        CharacterOptions3d {
            skin: 0.0,
            ..CharacterOptions3d::default()
        },
        CharacterOptions3d {
            max_slope_angle: -0.1,
            ..CharacterOptions3d::default()
        },
        CharacterOptions3d {
            step_height: -1.0,
            ..CharacterOptions3d::default()
        },
        CharacterOptions3d {
            snap_distance: f32::INFINITY,
            ..CharacterOptions3d::default()
        },
    ] {
        assert!(
            world
                .move_character(
                    shape,
                    at([0.0; 3]),
                    [1.0, 0.0, 0.0],
                    options,
                    RaycastFilter3d::default()
                )
                .is_err()
        );
    }
    assert!(
        world
            .move_character(
                Collider3d::sphere(-1.0).shape,
                at([0.0; 3]),
                [0.0; 3],
                CharacterOptions3d::default(),
                RaycastFilter3d::default()
            )
            .is_err()
    );
    assert!(
        world
            .move_character(
                Collider3d::cuboid([f32::MAX; 3]).shape,
                at([0.0; 3]),
                [0.0; 3],
                CharacterOptions3d::default(),
                RaycastFilter3d::default()
            )
            .is_err()
    );
    near3(world.pose(id(1)).unwrap().position, [2.0, 0.0, 0.0]);
    solid(&mut world, 2, [f32::MAX, 0.0, 0.0], [f32::MAX, 1.0, 1.0]);
    assert!(matches!(
        world.move_character(
            shape,
            at([0.0; 3]),
            [1.0, 0.0, 0.0],
            CharacterOptions3d::default(),
            RaycastFilter3d::default()
        ),
        Err(PhysicsError::NonFinite("character_obstacle_bounds"))
    ));
}

#[test]
fn sensor_inclusion_is_opt_in_and_pending_kinematic_targets_do_not_teleport() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_body(
            id(1),
            RigidBody3d {
                kind: RigidBodyKind::KinematicPosition,
                position: [2.0, 0.0, 0.0],
                ..RigidBody3d::default()
            },
            &[Collider3d {
                sensor: true,
                ..Collider3d::cuboid([0.5, 4.0, 4.0])
            }],
        )
        .unwrap();
    let query = |world: &PhysicsWorld3d| {
        world
            .move_character(
                Collider3d::sphere(0.2).shape,
                at([0.0; 3]),
                [4.0, 0.0, 0.0],
                CharacterOptions3d::default(),
                RaycastFilter3d {
                    include_sensors: true,
                    ..RaycastFilter3d::default()
                },
            )
            .unwrap()
    };
    near3(
        move_box(
            &world,
            at([0.0; 3]),
            [4.0, 0.0, 0.0],
            CharacterOptions3d::default(),
        )
        .translation,
        [4.0, 0.0, 0.0],
    );
    near3(query(&world).translation, [1.29, 0.0, 0.0]);
    world
        .set_kinematic_target(id(1), at([3.0, 0.0, 0.0]))
        .unwrap();
    near3(query(&world).translation, [1.29, 0.0, 0.0]);
    world.step(Duration::from_millis(16)).unwrap();
    near3(query(&world).translation, [2.29, 0.0, 0.0]);
}
