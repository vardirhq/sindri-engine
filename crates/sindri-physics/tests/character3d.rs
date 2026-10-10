//! Read-only 3D movement before scene/controller ownership is introduced.

use sindri_core::EntityId;
use sindri_physics::{
    CharacterMotion3d, CharacterOptions3d, Collider3d, ColliderShape3d, PhysicsPose3d,
    PhysicsWorld3d, RaycastFilter3d,
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
fn every_shape_moves_freely_in_xyz_without_a_body_or_step() {
    let world = PhysicsWorld3d::new([0.0, -9.81, 0.0]).unwrap();
    for shape in [
        Collider3d::sphere(0.5).shape,
        Collider3d::capsule(0.5, 0.25).shape,
        Collider3d::cuboid([0.2, 0.5, 0.2]).shape,
    ] {
        let result = world
            .move_character(
                shape,
                at([1.0, 2.0, 3.0]),
                [4.0, -2.0, 1.0],
                CharacterOptions3d::default(),
                RaycastFilter3d::default(),
            )
            .unwrap();
        near3(result.translation, [4.0, -2.0, 1.0]);
        assert!(!result.grounded);
        assert!(result.collisions.is_empty());
        assert!(world.is_empty());
    }
}

#[test]
fn walls_block_normal_motion_and_preserve_tangent_motion_in_three_dimensions() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    solid(&mut world, 1, [2.0, 0.0, 0.0], [0.5, 10.0, 10.0]);
    let result = move_box(
        &world,
        at([0.0; 3]),
        [4.0, 1.0, 2.0],
        CharacterOptions3d::default(),
    );
    near3(result.translation, [1.29, 1.0, 2.0]);
    let hit = result.collisions.first().unwrap();
    assert_eq!(hit.hit.entity, id(1));
    near3(hit.hit.normal, [-1.0, 0.0, 0.0]);
    assert!((hit.hit.point[0] - 1.5).abs() < 0.003);
    assert!(hit.hit.distance > 1.29);
    assert!(!result.grounded);
    let stopped = move_box(
        &world,
        at([0.0; 3]),
        [4.0, 1.0, 2.0],
        CharacterOptions3d {
            slide: false,
            ..CharacterOptions3d::default()
        },
    );
    assert!(stopped.translation[2] < 1.0);
}

#[test]
fn floor_contact_and_zero_motion_ground_all_shapes_without_consuming_physics() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    solid(&mut world, 1, [0.0, -0.25, 0.0], [10.0, 0.25, 10.0]);
    for (shape, half_height) in [
        (Collider3d::sphere(0.5).shape, 0.5),
        (Collider3d::capsule(0.5, 0.25).shape, 0.75),
        (Collider3d::cuboid([0.2, 0.5, 0.2]).shape, 0.5),
    ] {
        let result = world
            .move_character(
                shape,
                at([0.0, 3.0, 0.0]),
                [0.0, -4.0, 0.0],
                CharacterOptions3d::default(),
                RaycastFilter3d::default(),
            )
            .unwrap();
        assert!(
            (result.translation[1] - (half_height + 0.01 - 3.0)).abs() < 0.003,
            "{result:?}"
        );
        assert!(result.grounded);
        let standing = world
            .move_character(
                shape,
                at([0.0, half_height + 0.01, 0.0]),
                [0.0; 3],
                CharacterOptions3d::default(),
                RaycastFilter3d::default(),
            )
            .unwrap();
        near3(standing.translation, [0.0; 3]);
        assert!(standing.grounded);
    }
    near3(world.pose(id(1)).unwrap().position, [0.0, -0.25, 0.0]);
}

#[test]
fn snap_follows_a_lower_floor_and_upward_input_disables_it() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    solid(&mut world, 1, [-1.0, -0.25, 0.0], [1.0, 0.25, 2.0]);
    solid(&mut world, 2, [2.0, -0.45, 0.0], [2.0, 0.25, 2.0]);
    let pose = at([-0.5, 0.51, 0.0]);
    let options = CharacterOptions3d {
        snap_distance: 0.3,
        ..CharacterOptions3d::default()
    };
    let result = move_box(&world, pose, [2.0, 0.0, 0.0], options);
    near3(result.translation, [2.0, -0.2, 0.0]);
    assert!(result.grounded);
    let without_snap = move_box(&world, pose, [2.0, 0.0, 0.0], CharacterOptions3d::default());
    near3(without_snap.translation, [2.0, 0.0, 0.0]);
    assert!(!without_snap.grounded);
    let jump = move_box(&world, pose, [2.0, 0.3, 0.0], options);
    near3(jump.translation, [2.0, 0.3, 0.0]);
    assert!(!jump.grounded);
}

#[test]
fn optional_steps_climb_low_obstacles_but_reject_tall_steps_and_ceilings() {
    for (height, ceiling, accepted) in [(0.2, false, true), (0.6, false, false), (0.2, true, false)]
    {
        let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
        solid(&mut world, 1, [0.0, -0.25, 0.0], [10.0, 0.25, 10.0]);
        solid(
            &mut world,
            2,
            [0.8, height * 0.5, 0.0],
            [0.3, height * 0.5, 1.0],
        );
        if ceiling {
            solid(&mut world, 3, [0.0, 1.25, 0.0], [10.0, 0.1, 10.0]);
        }
        let pose = at([0.0, 0.51, 0.0]);
        let blocked = move_box(
            &world,
            pose,
            [1.0, -0.05, 0.0],
            CharacterOptions3d::default(),
        );
        assert!(blocked.translation[0] < 0.4);
        let result = move_box(
            &world,
            pose,
            [1.0, -0.05, 0.0],
            CharacterOptions3d {
                step_height: 0.3,
                step_min_width: 0.1,
                ..CharacterOptions3d::default()
            },
        );
        if accepted {
            assert!(
                result.translation[0] > 0.9 && result.translation[1] > 0.18,
                "{result:?}"
            );
            assert!(result.grounded);
        } else {
            assert!(result.translation[0] < 0.4, "{result:?}");
        }
    }
}

#[test]
fn orthogonal_walls_bound_corner_motion_without_losing_vertical_travel() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    solid(&mut world, 1, [2.0, 0.0, 0.0], [0.5, 10.0, 10.0]);
    solid(&mut world, 2, [0.0, 0.0, 2.0], [10.0, 10.0, 0.5]);
    let result = move_box(
        &world,
        at([0.0; 3]),
        [4.0, 1.0, 4.0],
        CharacterOptions3d::default(),
    );
    near3(result.translation, [1.29, 1.0, 1.29]);
    assert!(result.collisions.iter().any(|hit| hit.hit.entity == id(1)));
    assert!(result.collisions.iter().any(|hit| hit.hit.entity == id(2)));
}
