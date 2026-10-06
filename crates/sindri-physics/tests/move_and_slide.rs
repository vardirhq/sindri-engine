//! Geometric movement keeps policy out of the solver and does not mutate bodies.
#[path = "movement/support.rs"]
mod support;
use sindri_physics::{
    Collider2d, ColliderShape2d, CollisionLayers, PhysicsError, PhysicsPose2d, PhysicsWorld2d,
    RaycastFilter2d, SlideMotion2d, SlideOptions2d,
};
use support::{entity, insert, near, pose};

const PROBE: ColliderShape2d = ColliderShape2d::Circle { radius: 0.5 };

fn slide(world: &PhysicsWorld2d, start: PhysicsPose2d, movement: [f32; 2]) -> SlideMotion2d {
    world
        .move_and_slide(
            PROBE,
            start,
            movement,
            SlideOptions2d::default(),
            RaycastFilter2d::default(),
        )
        .unwrap()
}

#[test]
fn empty_space_and_zero_motion_work_for_every_shape_without_a_step() {
    let world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    for shape in [
        PROBE,
        ColliderShape2d::Box {
            half_extents: [0.4, 0.7],
        },
        ColliderShape2d::Capsule {
            half_height: 0.4,
            radius: 0.3,
        },
    ] {
        for movement in [[3.0, -2.0], [0.0; 2]] {
            let result = world
                .move_and_slide(
                    shape,
                    pose(10.0, 5.0),
                    movement,
                    SlideOptions2d::default(),
                    RaycastFilter2d::default(),
                )
                .unwrap();
            near(result.translation[0], movement[0]);
            near(result.translation[1], movement[1]);
            assert!(result.collisions.is_empty());
            assert!(!result.started_penetrating && !result.iteration_limit_reached);
        }
    }
}

#[test]
fn a_wall_keeps_skin_and_the_full_tangential_movement() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(3.0, 0.0),
        &[Collider2d::rectangle([0.5, 10.0])],
    );
    let result = slide(&world, pose(0.0, 0.0), [4.0, 3.0]);
    near(result.translation[0], 1.99);
    near(result.translation[1], 3.0);
    assert_eq!(result.collisions.len(), 1);
    near(result.collisions[0].normal[0], -1.0);
    assert!(!result.iteration_limit_reached);
    let endpoint = pose(result.translation[0], result.translation[1]);
    assert!(
        world
            .overlap(PROBE, endpoint, RaycastFilter2d::default())
            .unwrap()
            .is_empty()
    );
    // The movement query never teleports the obstacle.
    near(
        slide(&world, pose(0.0, 0.0), [4.0, 0.0]).translation[0],
        1.99,
    );
}

#[test]
fn touching_and_inside_skin_allow_tangent_and_escape_but_block_approach() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(0.0, -1.0),
        &[Collider2d::rectangle([10.0, 0.5])],
    );
    for y in [0.0, 0.005, 0.01] {
        let tangent = slide(&world, pose(0.0, y), [2.0, 0.0]);
        near(tangent.translation[0], 2.0);
        near(slide(&world, pose(0.0, y), [0.0, 1.0]).translation[1], 1.0);
        let approach = slide(&world, pose(0.0, y), [1.0, -1.0]);
        near(approach.translation[0], 1.0);
        near(approach.translation[1], 0.0);
        assert!(!approach.started_penetrating && !approach.iteration_limit_reached);
    }
}

#[test]
fn initial_penetration_is_reported_deterministically_without_recovery() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    for id in [9, 2] {
        insert(
            &mut world,
            id,
            pose(0.0, 0.0),
            &[Collider2d::rectangle([1.0, 1.0])],
        );
    }
    for movement in [[2.0, 1.0], [0.0; 2]] {
        let result = slide(&world, pose(0.0, 0.0), movement);
        assert!(result.started_penetrating);
        assert_eq!(result.collisions[0].entity, entity(2));
        near(result.translation[0], 0.0);
        near(result.translation[1], 0.0);
        near(result.remaining[0], movement[0]);
        near(result.remaining[1], movement[1]);
        assert!(!result.iteration_limit_reached);
    }
}

#[test]
fn a_corner_blocks_both_axes_and_a_small_budget_reports_unapplied_slide() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(3.0, 0.0),
        &[Collider2d::rectangle([0.5, 10.0])],
    );
    insert(
        &mut world,
        2,
        pose(0.0, 3.0),
        &[Collider2d::rectangle([10.0, 0.5])],
    );
    let result = slide(&world, pose(0.0, 0.0), [4.0, 3.0]);
    near(result.translation[0], 1.99);
    near(result.translation[1], 1.99);
    assert_eq!(result.collisions.len(), 2);
    assert!(!result.iteration_limit_reached);
    let limited = world
        .move_and_slide(
            PROBE,
            pose(0.0, 0.0),
            [4.0, 3.0],
            SlideOptions2d {
                max_iterations: 1,
                ..SlideOptions2d::default()
            },
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(limited.iteration_limit_reached);
    assert!(limited.remaining[1] > 1.0);
    assert_eq!(limited.collisions.len(), 1);
}

#[test]
fn rotated_surfaces_slide_in_their_world_tangent() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    let slope = PhysicsPose2d {
        rotation: std::f32::consts::FRAC_PI_4,
        ..pose(0.0, 0.0)
    };
    insert(&mut world, 1, slope, &[Collider2d::rectangle([10.0, 0.1])]);
    let result = slide(&world, pose(0.0, 2.0), [3.0, -2.0]);
    assert!(!result.started_penetrating);
    assert!(!result.collisions.is_empty());
    assert!(result.collisions.iter().all(|hit| hit.entity == entity(1)));
    // Project the endpoint onto the slope tangent, retaining the radius and
    // skin above the surface. Narrow-phase normals can need a refinement cast.
    let separation = (0.1 + 0.5 + 0.01) * 2.0_f32.sqrt();
    near(result.translation[0], (3.0 - separation) * 0.5);
    near(result.translation[1], (3.0 + separation) * 0.5 - 2.0);
    assert!(!result.iteration_limit_reached);
    let at = pose(result.translation[0], 2.0 + result.translation[1]);
    assert!(
        world
            .overlap(PROBE, at, RaycastFilter2d::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn filters_exclude_whole_compounds_sensors_masks_and_host_inactive_entities() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    let sensor = Collider2d {
        sensor: true,
        ..Collider2d::rectangle([0.2, 10.0])
    };
    insert(&mut world, 1, pose(1.0, 0.0), &[sensor]);
    let wall = Collider2d {
        layers: CollisionLayers {
            memberships: 2,
            filter: u32::MAX,
        },
        ..Collider2d::rectangle([0.2, 10.0])
    };
    insert(
        &mut world,
        2,
        pose(3.0, 0.0),
        &[
            wall,
            Collider2d {
                offset: [0.2, 0.0],
                ..wall
            },
        ],
    );
    insert(&mut world, 3, pose(5.0, 0.0), &[wall]);
    let result = world
        .move_and_slide_where(
            PROBE,
            pose(0.0, 0.0),
            [8.0, 0.0],
            SlideOptions2d::default(),
            RaycastFilter2d {
                exclude: Some(entity(2)),
                ..RaycastFilter2d::default()
            },
            |id| id != entity(3),
        )
        .unwrap();
    near(result.translation[0], 8.0);
    let result = world
        .move_and_slide(
            PROBE,
            pose(0.0, 0.0),
            [8.0, 0.0],
            SlideOptions2d::default(),
            RaycastFilter2d {
                mask: 1,
                ..RaycastFilter2d::default()
            },
        )
        .unwrap();
    near(result.translation[0], 8.0);
    let result = world
        .move_and_slide(
            PROBE,
            pose(0.0, 0.0),
            [8.0, 0.0],
            SlideOptions2d::default(),
            RaycastFilter2d {
                include_sensors: true,
                ..RaycastFilter2d::default()
            },
        )
        .unwrap();
    assert_eq!(result.collisions[0].entity, entity(1));
    near(result.translation[0], 0.29);
}

#[test]
fn immediate_teleports_and_removal_change_movement_without_a_step() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(3.0, 0.0),
        &[Collider2d::rectangle([0.5, 10.0])],
    );
    world.move_to(entity(1), pose(5.0, 0.0)).unwrap();
    near(
        slide(&world, pose(0.0, 0.0), [8.0, 0.0]).translation[0],
        3.99,
    );
    world.remove(entity(1));
    near(
        slide(&world, pose(0.0, 0.0), [8.0, 0.0]).translation[0],
        8.0,
    );
}

#[test]
fn invalid_inputs_are_rejected_even_for_zero_movement() {
    let world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    for count in [0, 33, 255] {
        assert_eq!(
            world.move_and_slide(
                PROBE,
                pose(0.0, 0.0),
                [0.0; 2],
                SlideOptions2d {
                    max_iterations: count,
                    ..SlideOptions2d::default()
                },
                RaycastFilter2d::default()
            ),
            Err(PhysicsError::InvalidSlideIterations)
        );
    }
    for skin in [-1.0, 0.0, f32::NAN, f32::INFINITY] {
        assert!(
            world
                .move_and_slide(
                    PROBE,
                    pose(0.0, 0.0),
                    [0.0; 2],
                    SlideOptions2d {
                        skin,
                        ..SlideOptions2d::default()
                    },
                    RaycastFilter2d::default()
                )
                .is_err()
        );
    }
    assert!(
        world
            .move_and_slide(
                PROBE,
                pose(f32::MAX, 0.0),
                [f32::MAX, 0.0],
                SlideOptions2d::default(),
                RaycastFilter2d::default()
            )
            .is_err()
    );
    assert!(
        world
            .move_and_slide(
                PROBE,
                pose(0.0, 0.0),
                [f32::NAN, 0.0],
                SlideOptions2d::default(),
                RaycastFilter2d::default()
            )
            .is_err()
    );
    assert!(
        world
            .move_and_slide(
                ColliderShape2d::Circle { radius: 0.0 },
                pose(0.0, 0.0),
                [0.0; 2],
                SlideOptions2d::default(),
                RaycastFilter2d::default()
            )
            .is_err()
    );
}

#[test]
fn exact_sweep_ties_prefer_entity_then_original_piece_order() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    for id in [9, 2] {
        insert(
            &mut world,
            id,
            pose(3.0, 0.0),
            &[Collider2d::rectangle([0.5, 10.0])],
        );
    }
    let result = slide(&world, pose(0.0, 0.0), [4.0, 0.0]);
    assert_eq!(result.collisions[0].entity, entity(2));
    // A corner in one compound has equal-distance contacts with two normals.
    // The first piece is the horizontal surface, so it is the first reported hit.
    let mut compound = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut compound,
        1,
        pose(0.0, 0.0),
        &[
            Collider2d {
                offset: [0.0, 3.0],
                ..Collider2d::rectangle([10.0, 0.5])
            },
            Collider2d {
                offset: [3.0, 0.0],
                ..Collider2d::rectangle([0.5, 10.0])
            },
        ],
    );
    let result = slide(&compound, pose(0.0, 0.0), [4.0, 4.0]);
    near(result.collisions[0].normal[1], -1.0);
    near(result.translation[0], 1.99);
    near(result.translation[1], 1.99);
}

#[test]
fn one_way_geometry_remains_solid_for_this_geometric_primitive() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(0.0, 0.0),
        &[Collider2d::rectangle([10.0, 0.5])],
    );
    world
        .set_one_way(entity(1), Some(sindri_physics::OneWay2d::default()))
        .unwrap();
    for (start, movement, expected) in [(-2.0, 3.0, 0.99), (2.0, -3.0, -0.99)] {
        let result = slide(&world, pose(0.0, start), [0.0, movement]);
        near(result.translation[1], expected);
        assert_eq!(result.collisions[0].entity, entity(1));
    }
}

#[test]
fn flat_surface_contact_refines_skin_cast_normals_without_lifting_a_box() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(0.0, -1.0),
        &[Collider2d::rectangle([10.0, 0.5])],
    );
    for shape in [
        ColliderShape2d::Box {
            half_extents: [0.4, 0.5],
        },
        ColliderShape2d::Circle { radius: 0.5 },
        ColliderShape2d::Capsule {
            half_height: 0.2,
            radius: 0.3,
        },
    ] {
        let mut at = pose(0.0, 0.01);
        for _ in 0..3 {
            let result = world
                .move_and_slide(
                    shape,
                    at,
                    [1.5, 0.0],
                    SlideOptions2d::default(),
                    RaycastFilter2d::default(),
                )
                .unwrap();
            near(result.translation[0], 1.5);
            near(result.translation[1], 0.0);
            assert!(!result.started_penetrating && !result.iteration_limit_reached);
            at.position[0] += result.translation[0];
            at.position[1] += result.translation[1];
        }
    }
}
