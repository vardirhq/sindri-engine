//! Controller query equivalence, skin bounds, historical support and sparse work.

use super::{id, populated, pose, with_scan};
use crate::{
    Collider2d, ColliderShape2d, GroundOptions2d, GroundedSlideOptions2d, OneWay2d, PhysicsWorld2d,
    PlatformSupport2d, RaycastFilter2d, SlideOptions2d,
};
use std::collections::HashSet;

#[test]
fn controller_results_match_exhaustive_candidates_across_shape_skin_and_policy() {
    let mut world = populated();
    world.set_one_way(id(7), Some(OneWay2d::default())).unwrap();
    for shape in [
        ColliderShape2d::Circle { radius: 0.4 },
        ColliderShape2d::Box {
            half_extents: [0.4, 0.7],
        },
        ColliderShape2d::Capsule {
            half_height: 0.5,
            radius: 0.3,
        },
    ] {
        for i in 0..12_u16 {
            for skin in [0.01, 0.5] {
                for drop_through in [false, true] {
                    let at = pose(f32::from(i) - 7.0, f32::from(i) * 0.8 - 5.0, 0.43);
                    let up = if i % 2 == 0 { [0.0, 1.0] } else { [1.0, 0.0] };
                    let options = GroundedSlideOptions2d {
                        slide: SlideOptions2d {
                            skin,
                            ..SlideOptions2d::default()
                        },
                        up,
                        snap_distance: 0.4,
                        step_height: 0.4,
                        drop_through,
                        ..GroundedSlideOptions2d::default()
                    };
                    let filter = RaycastFilter2d {
                        exclude: Some(id(3)),
                        ..RaycastFilter2d::default()
                    };
                    let read = |world: &PhysicsWorld2d| {
                        (
                            world
                                .move_and_slide_grounded_where(
                                    shape,
                                    at,
                                    [3.0, -1.0],
                                    options,
                                    filter,
                                    |entity| entity != id(4),
                                )
                                .unwrap(),
                            world
                                .probe_ground_where(
                                    shape,
                                    at,
                                    GroundOptions2d {
                                        skin,
                                        up,
                                        max_distance: 0.7,
                                        ..GroundOptions2d::default()
                                    },
                                    filter,
                                    |entity| entity != id(4),
                                )
                                .unwrap(),
                        )
                    };
                    let indexed = read(&world);
                    assert_eq!(
                        indexed,
                        with_scan(&mut world, read),
                        "probe {i}, skin {skin}, drop {drop_through}"
                    );
                }
            }
        }
    }
}

#[test]
fn zero_travel_skin_support_includes_separated_geometry() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    world
        .insert_static_collider(
            id(1),
            pose(0.0, 0.0, 0.0),
            &[Collider2d::rectangle([2.0, 0.25])],
        )
        .unwrap();
    // Probe bottom is 0.15 above the floor; the shapes' AABBs do not overlap.
    let ground = world
        .probe_ground(
            ColliderShape2d::Circle { radius: 0.5 },
            pose(0.0, 0.9, 0.0),
            GroundOptions2d {
                skin: 0.2,
                max_distance: 0.0,
                ..GroundOptions2d::default()
            },
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(ground.walkable);
    assert_eq!(ground.hit.unwrap().entity, id(1));
    assert!(ground.hit.unwrap().distance.abs() < f32::EPSILON);
}

#[test]
fn support_reconstruction_uses_previous_pose_even_outside_current_bounds() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    world
        .insert_static_collider(
            id(1),
            pose(0.0, 0.0, 0.0),
            &[Collider2d::rectangle([2.0, 0.25])],
        )
        .unwrap();
    world.move_to(id(1), pose(100.0, 0.0, 0.0)).unwrap();
    let options = GroundedSlideOptions2d {
        platform_support: Some(PlatformSupport2d {
            entity: id(1),
            previous_pose: pose(0.0, 0.0, 0.0),
        }),
        ..GroundedSlideOptions2d::default()
    };
    let motion = world
        .move_and_slide_grounded(
            ColliderShape2d::Circle { radius: 0.5 },
            pose(0.0, 0.76, 0.0),
            [0.0; 2],
            options,
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(motion.platform.is_some());
    assert!((motion.translation[0] - 100.0).abs() < 0.0001);
    assert!(motion.grounded);
}

#[test]
fn sparse_controller_calls_visit_only_local_entities_as_the_world_grows() {
    for count in [100_u16, 1_000, 10_000] {
        let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
        for i in 0..count {
            world
                .insert_static_collider(
                    id(u32::from(i)),
                    pose(f32::from(i) * 4.0, 0.0, 0.0),
                    &[Collider2d::rectangle([0.5, 0.25])],
                )
                .unwrap();
        }
        for sample in [0, count / 2, count - 1] {
            let mut visited = HashSet::new();
            let motion = world
                .move_and_slide_grounded_where(
                    ColliderShape2d::Circle { radius: 0.3 },
                    pose(f32::from(sample) * 4.0, 0.56, 0.0),
                    [0.1, 0.0],
                    GroundedSlideOptions2d {
                        snap_distance: 0.1,
                        ..GroundedSlideOptions2d::default()
                    },
                    RaycastFilter2d::default(),
                    |entity| {
                        visited.insert(entity);
                        true
                    },
                )
                .unwrap();
            assert!(motion.grounded);
            assert!((motion.translation[0] - 0.1).abs() < 0.003);
            assert_eq!(visited, HashSet::from([id(u32::from(sample))]));
            eprintln!("{count} pieces, controller sample {sample}: 1 candidate entity");
        }
    }
}
