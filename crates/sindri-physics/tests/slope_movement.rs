//! Slope classification must also constrain the grounded movement path.
#[path = "movement/support.rs"]
mod support;
use sindri_physics::{
    Collider2d, ColliderShape2d, GroundedSlideMotion2d, GroundedSlideOptions2d, PhysicsPose2d,
    PhysicsWorld2d, RaycastFilter2d, SlideOptions2d,
};
use support::{entity, insert, near, pose};

const PROBE: ColliderShape2d = ColliderShape2d::Circle { radius: 0.5 };

fn slope(angle: f32) -> (PhysicsWorld2d, PhysicsPose2d) {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    let rotation = angle.to_radians();
    insert(
        &mut world,
        1,
        PhysicsPose2d {
            position: [0.0; 2],
            rotation,
        },
        &[Collider2d::rectangle([10.0, 0.5])],
    );
    (world, pose(0.0, 1.01 / rotation.cos()))
}

fn move_probe(
    world: &PhysicsWorld2d,
    at: PhysicsPose2d,
    displacement: [f32; 2],
    limit: f32,
) -> GroundedSlideMotion2d {
    world
        .move_and_slide_grounded(
            PROBE,
            at,
            displacement,
            GroundedSlideOptions2d {
                max_slope_angle: limit.to_radians(),
                snap_distance: 0.02,
                ..GroundedSlideOptions2d::default()
            },
            RaycastFilter2d::default(),
        )
        .unwrap()
}

#[test]
fn horizontal_motion_climbs_only_walkable_slopes_including_the_limit() {
    for (angle, walkable) in [(0.0, true), (30.0, true), (45.0, true), (60.0, false)] {
        let (world, at) = slope(angle);
        let result = move_probe(&world, at, [0.2, 0.0], 45.0);
        assert_eq!(
            result.ground.walkable, walkable,
            "angle {angle}: {result:?}"
        );
        if walkable {
            assert!(result.translation[0] > 0.09, "{result:?}");
            if angle > 0.0 {
                assert!(result.translation[1] > 0.05);
            }
        } else {
            assert!(!result.grounded && !result.slide.iteration_limit_reached);
            assert!(!result.slide.collisions.is_empty());
            assert!(result.translation[0] < 0.002);
            near(result.translation[1], 0.0);
            near(result.slide.remaining[0], 0.0);
        }
    }
}

#[test]
fn configurable_limit_changes_motion_and_support_together() {
    let (world, at) = slope(45.0);
    let blocked = move_probe(&world, at, [0.2, 0.0], 40.0);
    near(blocked.translation[0], 0.0);
    assert!(!blocked.grounded);
    let allowed = move_probe(&world, at, [0.2, 0.0], 50.0);
    assert!(allowed.translation[0] > 0.09 && allowed.translation[1] > 0.09);
    assert!(allowed.grounded);
}

#[test]
fn ordinary_geometric_sliding_keeps_its_unrestricted_projection() {
    let (world, at) = slope(60.0);
    let before = world.pose(entity(1)).unwrap();
    let geometric = world
        .move_and_slide(
            PROBE,
            at,
            [0.2, 0.0],
            SlideOptions2d::default(),
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(geometric.translation[0] > 0.04 && geometric.translation[1] > 0.08);
    let grounded = move_probe(&world, at, [0.2, 0.0], 45.0);
    near(grounded.translation[0], 0.0);
    assert_eq!(world.pose(entity(1)).unwrap(), before);
}

#[test]
fn explicit_ascent_can_slide_without_gaining_unrequested_rise() {
    let (world, at) = slope(60.0);
    let result = move_probe(&world, at, [0.2, 0.1], 45.0);
    assert!(!result.grounded && !result.slide.started_penetrating);
    assert!(result.translation[0] > 0.04 && result.translation[1] > 0.09);
    assert!(result.translation[1] <= 0.1002, "{result:?}");
    near(result.snap_translation[1], 0.0);
    let endpoint = pose(
        at.position[0] + result.translation[0],
        at.position[1] + result.translation[1],
    );
    assert!(
        !move_probe(&world, endpoint, [0.0; 2], 45.0)
            .slide
            .started_penetrating
    );
}

#[test]
fn steep_downward_sliding_is_allowed_without_grounding_or_snapping() {
    let (world, at) = slope(60.0);
    let result = world
        .move_and_slide_grounded(
            PROBE,
            at,
            [0.0, -0.2],
            GroundedSlideOptions2d {
                snap_distance: 0.5,
                ..GroundedSlideOptions2d::default()
            },
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(!result.grounded && !result.ground.walkable);
    assert!(result.translation[0] < -0.08 && result.translation[1] < -0.14);
    near(result.snap_translation[1], 0.0);
    near(result.slide.remaining[1], 0.0);
}

#[test]
fn negative_vertical_requests_cannot_be_redirected_into_a_steep_climb() {
    let (world, at) = slope(60.0);
    let result = move_probe(&world, at, [0.2, -0.01], 45.0);
    assert!(!result.grounded);
    assert!(result.translation[1] <= 0.0001);
    assert!(result.translation[0] < 0.002);
}

#[test]
fn wall_and_ceiling_contacts_do_not_gain_ground_slope_restrictions() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    insert(
        &mut world,
        1,
        pose(1.0, 0.0),
        &[Collider2d::rectangle([0.5, 10.0])],
    );
    let wall = move_probe(&world, pose(-0.01, 0.0), [0.1, 0.2], 0.0);
    near(wall.translation[0], 0.0);
    near(wall.translation[1], 0.2);
    assert!(!wall.grounded);
    world.remove(entity(1));
    insert(
        &mut world,
        2,
        pose(0.0, 1.0),
        &[Collider2d::rectangle([10.0, 0.5])],
    );
    let ceiling = move_probe(&world, pose(0.0, -0.01), [0.2, 0.1], 0.0);
    near(ceiling.translation[0], 0.2);
    near(ceiling.translation[1], 0.0);
    assert!(!ceiling.grounded);
}

#[test]
fn rotated_up_limits_world_space_rise_with_a_rotated_box_probe() {
    let mut world = PhysicsWorld2d::new([0.0; 2]).unwrap();
    let rotation = 150.0_f32.to_radians();
    insert(
        &mut world,
        1,
        PhysicsPose2d {
            position: [0.0; 2],
            rotation,
        },
        &[Collider2d::rectangle([10.0, 0.5])],
    );
    // Rotating the box by pi/2 swaps its world-space half extents.
    let extent = 0.5 * 0.6 + 60.0_f32.to_radians().sin() * 0.4;
    let result = world
        .move_and_slide_grounded(
            ColliderShape2d::Box {
                half_extents: [0.4, 0.6],
            },
            PhysicsPose2d {
                position: [-(0.5 + extent + 0.01) / 0.5, 0.0],
                rotation: std::f32::consts::FRAC_PI_2,
            },
            [0.0, 0.2],
            GroundedSlideOptions2d {
                up: [-1.0, 0.0],
                ..GroundedSlideOptions2d::default()
            },
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(!result.grounded && !result.slide.started_penetrating);
    assert!(!result.slide.collisions.is_empty());
    near(result.translation[0], 0.0);
    near(result.translation[1], 0.0);
}

#[test]
fn filtered_steep_surfaces_do_not_restrict_motion() {
    let (world, at) = slope(60.0);
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
            .move_and_slide_grounded(
                PROBE,
                at,
                [0.2, 0.0],
                GroundedSlideOptions2d::default(),
                filter,
            )
            .unwrap();
        near(result.translation[0], 0.2);
        assert!(result.slide.collisions.is_empty());
    }
    let inactive = world
        .move_and_slide_grounded_where(
            PROBE,
            at,
            [0.2, 0.0],
            GroundedSlideOptions2d::default(),
            RaycastFilter2d::default(),
            |_| false,
        )
        .unwrap();
    near(inactive.translation[0], 0.2);
    assert!(inactive.slide.collisions.is_empty());
}

#[test]
fn mirrored_steep_slope_blocks_a_capsule_without_changing_probe_rotation() {
    let (world, at) = slope(-60.0);
    let result = world
        .move_and_slide_grounded(
            ColliderShape2d::Capsule {
                half_height: 0.4,
                radius: 0.3,
            },
            at,
            [-0.2, 0.0],
            GroundedSlideOptions2d::default(),
            RaycastFilter2d::default(),
        )
        .unwrap();
    assert!(!result.grounded && !result.slide.started_penetrating);
    assert!(!result.slide.collisions.is_empty());
    near(result.translation[0], 0.0);
    near(result.translation[1], 0.0);
}

#[test]
fn downhill_following_snaps_only_to_support_inside_the_same_slope_limit() {
    for (angle, walkable) in [(30.0, true), (60.0, false)] {
        let (world, at) = slope(angle);
        let result = world
            .move_and_slide_grounded(
                PROBE,
                at,
                [-0.2, 0.0],
                GroundedSlideOptions2d {
                    snap_distance: 0.5,
                    ..GroundedSlideOptions2d::default()
                },
                RaycastFilter2d::default(),
            )
            .unwrap();
        near(result.slide.translation[0], -0.2);
        near(result.slide.translation[1], 0.0);
        assert_eq!(result.grounded, walkable);
        assert_eq!(result.ground.walkable, walkable);
        assert!(result.ground.hit.is_some());
        if walkable {
            near(result.translation[1], -0.2 * angle.to_radians().tan());
        } else {
            near(result.translation[1], 0.0);
        }
    }
}
