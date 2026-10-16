use super::*;
use crate::{
    BodyControl3d, ColliderShape3d, ConvexHull3d, ConvexHullError3d, PhysicsEventKind,
    RaycastFilter3d, RigidBodyKind,
};
use std::time::Duration;

fn id(value: u64) -> EntityId {
    EntityId::from_bits(value << 32)
}

fn cube_points() -> [[f32; 3]; 8] {
    [
        [-0.5, -0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [-0.5, 0.5, -0.5],
        [-0.5, 0.5, 0.5],
        [0.5, -0.5, -0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, -0.5],
        [0.5, 0.5, 0.5],
    ]
}

fn cube() -> ConvexHullCollider3d {
    ConvexHullCollider3d::new(ConvexHull3d::new(&cube_points()).unwrap())
}

fn at(position: [f32; 3]) -> PhysicsPose3d {
    PhysicsPose3d {
        position,
        ..PhysicsPose3d::default()
    }
}

fn ray(world: &PhysicsWorld3d, x: f32) -> Option<crate::RayHit3d> {
    world
        .raycast(
            [x, 5.0, 0.0],
            [0.0, -1.0, 0.0],
            10.0,
            RaycastFilter3d::default(),
        )
        .unwrap()
}

#[test]
fn construction_rejects_invalid_and_nonvolumetric_input() {
    assert!(matches!(
        ConvexHull3d::new(&cube_points()[..3]),
        Err(ConvexHullError3d::TooFewPoints)
    ));
    assert!(matches!(
        ConvexHull3d::new(&vec![[0.0; 3]; 4097]),
        Err(ConvexHullError3d::Limit)
    ));
    for value in [f32::NAN, f32::INFINITY, 10_001.0] {
        let mut points = cube_points();
        points[3][1] = value;
        assert!(matches!(
            ConvexHull3d::new(&points),
            Err(ConvexHullError3d::Point(3))
        ));
    }
    for points in [
        vec![[0.0; 3]; 8],
        vec![[0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [3.0, 0.0, 0.0]],
        vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 1.0]],
        cube_points().map(|p| p.map(|v| v * 1.0e-20)).to_vec(),
    ] {
        assert!(
            matches!(
                ConvexHull3d::new(&points),
                Err(ConvexHullError3d::Degenerate)
            ),
            "{points:?}"
        );
    }
    let mut points = cube_points().to_vec();
    points.extend([[0.0; 3], cube_points()[0]]);
    assert!(ConvexHull3d::new(&points).is_ok());
    assert!(ConvexHull3d::new(&vec![cube_points()[0]; 4096]).is_err());
}

#[test]
fn hull_surface_is_convex_geometry_and_clones_share_it() {
    let geometry =
        ConvexHull3d::new(&[[0.0; 3], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 2.0]]).unwrap();
    let copy = geometry.clone();
    assert!(std::sync::Arc::ptr_eq(&geometry.shape.0, &copy.shape.0));
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_hull_body(
            id(1),
            RigidBody3d {
                kind: RigidBodyKind::Static,
                ..RigidBody3d::default()
            },
            &[],
            &[ConvexHullCollider3d::new(geometry)],
        )
        .unwrap();
    let hit = world
        .raycast(
            [0.25, 3.0, 0.25],
            [0.0, -1.0, 0.0],
            4.0,
            RaycastFilter3d::default(),
        )
        .unwrap()
        .unwrap();
    assert!((hit.point[1] - 1.5).abs() < 0.001);
    assert!(
        world
            .raycast(
                [1.5, 3.0, 1.5],
                [0.0, -1.0, 0.0],
                4.0,
                RaycastFilter3d::default()
            )
            .unwrap()
            .is_none()
    );
    assert_eq!(
        world
            .overlap(
                ColliderShape3d::Sphere { radius: 0.1 },
                at([0.2; 3]),
                RaycastFilter3d::default()
            )
            .unwrap(),
        [id(1)]
    );
    world.move_to(id(1), at([10.0, 0.0, 0.0])).unwrap();
    assert!(
        world
            .raycast(
                [0.25, 3.0, 0.25],
                [0.0, -1.0, 0.0],
                4.0,
                RaycastFilter3d::default()
            )
            .unwrap()
            .is_none()
    );
    assert!(
        world
            .raycast(
                [10.25, 3.0, 0.25],
                [0.0, -1.0, 0.0],
                4.0,
                RaycastFilter3d::default()
            )
            .unwrap()
            .is_some()
    );
}

#[test]
fn dynamic_hull_lands_and_impulses_replay_after_pending_insertion() {
    let mut world = PhysicsWorld3d::new([0.0, -9.81, 0.0]).unwrap();
    world
        .insert_static_collider(
            id(1),
            at([0.0, -0.25, 0.0]),
            &[Collider3d::cuboid([10.0, 0.25, 10.0])],
        )
        .unwrap();
    world
        .remember_control(
            id(2),
            RigidBodyKind::Dynamic,
            BodyControl3d::Impulse([1.0, 0.0, 0.0]),
        )
        .unwrap();
    world
        .insert_hull_body(
            id(2),
            RigidBody3d {
                position: [0.0, 2.0, 0.0],
                lock_rotation: true,
                ..RigidBody3d::default()
            },
            &[],
            &[cube()],
        )
        .unwrap();
    assert!((world.mass(id(2)).unwrap() - 1.0).abs() < 0.001);
    assert!((world.linear_velocity(id(2)).unwrap()[0] - 1.0).abs() < 0.001);
    assert!(!world.pending_controls.contains_key(&id(2)));
    let mut copy = world.clone();
    let mut landed = false;
    for _ in 0..180 {
        let dt = Duration::from_secs_f32(1.0 / 60.0);
        let events = world.step(dt).unwrap();
        assert_eq!(events, copy.step(dt).unwrap());
        landed |= events.iter().any(|event| {
            event.first == id(1)
                && event.second == id(2)
                && event.kind == PhysicsEventKind::CollisionStarted
        });
    }
    let pose = world.pose(id(2)).unwrap();
    assert!((pose.position[1] - 0.5).abs() < 0.03, "{pose:?}");
    assert!(pose.position[0] > 0.1);
    assert_eq!(pose, copy.pose(id(2)).unwrap());
    assert!(landed);
    world.apply_impulse(id(2), [0.0, 2.0, 0.0]).unwrap();
    assert!(world.linear_velocity(id(2)).unwrap()[1] > 1.9);
}

#[test]
fn compounds_sum_primitive_and_rotated_offset_hull_mass() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let mut hull = cube();
    hull.pose = at([3.0, 0.0, 0.0]);
    let angle = std::f32::consts::FRAC_PI_4;
    hull.pose.rotation = [0.0, 0.0, angle.sin(), angle.cos()];
    world
        .insert_hull_body(
            id(1),
            RigidBody3d::default(),
            &[Collider3d::cuboid([0.5; 3])],
            &[hull],
        )
        .unwrap();
    assert!((world.mass(id(1)).unwrap() - 2.0).abs() < 0.001);
    assert!(ray(&world, 0.0).is_some());
    assert!(ray(&world, 3.0).is_some());
    assert!(world.remove(id(1)));
    assert!(ray(&world, 0.0).is_none() && ray(&world, 3.0).is_none());
}

#[test]
fn invalid_compound_retains_pending_controls_and_never_inserts() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .remember_control(
            id(1),
            RigidBodyKind::Dynamic,
            BodyControl3d::LinearVelocity([1.0, 0.0, 0.0]),
        )
        .unwrap();
    let mut bad = cube();
    bad.material.friction = f32::NAN;
    assert!(matches!(
        world.insert_hull_body(
            id(1),
            RigidBody3d::default(),
            &[Collider3d::sphere(0.5)],
            &[cube(), bad]
        ),
        Err(PhysicsError::ColliderPiece { index: 2, .. })
    ));
    assert!(!world.contains(id(1)) && ray(&world, 0.0).is_none());
    assert!(world.pending_controls.contains_key(&id(1)));
    let mut bad = cube();
    bad.pose = at([f32::MAX, 0.0, 0.0]);
    assert!(
        world
            .insert_hull_body(id(1), RigidBody3d::default(), &[], &[cube(), bad])
            .is_err()
    );
    assert!(!world.contains(id(1)));
    assert!(matches!(
        world.insert_hull_body(
            id(1),
            RigidBody3d::default(),
            &[Collider3d::sphere(f32::MAX)],
            &[cube()]
        ),
        Err(PhysicsError::ColliderPiece { index: 0, .. })
    ));
    assert!(matches!(
        world.insert_hull_body(id(1), RigidBody3d::default(), &[], &[]),
        Err(PhysicsError::NoColliderPieces(_))
    ));
    assert!(matches!(
        world.insert_hull_body(
            id(1),
            RigidBody3d::default(),
            &vec![Collider3d::sphere(1.0); 1025],
            &[]
        ),
        Err(PhysicsError::TooManyColliderPieces { .. })
    ));
    assert!(
        world
            .insert_hull_body(
                id(1),
                RigidBody3d {
                    kind: RigidBodyKind::Static,
                    ..RigidBody3d::default()
                },
                &[],
                &[cube()]
            )
            .is_err()
    );
    world
        .insert_hull_body(id(1), RigidBody3d::default(), &[], &[cube()])
        .unwrap();
    let original = world.pose(id(1)).unwrap();
    assert!(matches!(
        world.insert_hull_body(id(1), RigidBody3d::default(), &[], &[cube()]),
        Err(PhysicsError::EntityAlreadyRegistered(_))
    ));
    assert_eq!(world.pose(id(1)).unwrap(), original);
}

#[test]
fn kinematic_hulls_follow_both_target_and_velocity_controls() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    for (entity, kind) in [
        (id(1), RigidBodyKind::KinematicPosition),
        (id(2), RigidBodyKind::KinematicVelocity),
    ] {
        world
            .insert_hull_body(
                entity,
                RigidBody3d {
                    kind,
                    ..RigidBody3d::default()
                },
                &[],
                &[cube()],
            )
            .unwrap();
    }
    world.move_to(id(1), at([3.0, 0.0, 0.0])).unwrap();
    world.set_linear_velocity(id(2), [-3.0, 0.0, 0.0]).unwrap();
    world.step(Duration::from_secs(1)).unwrap();
    assert!((world.pose(id(1)).unwrap().position[0] - 3.0).abs() < 0.001);
    assert!((world.pose(id(2)).unwrap().position[0] + 3.0).abs() < 0.001);
    assert_eq!(ray(&world, 3.0).unwrap().entity, id(1));
    assert_eq!(ray(&world, -3.0).unwrap().entity, id(2));
}

#[test]
fn sensor_hulls_keep_masks_exclusions_and_enter_exit_events() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let mut sensor = cube();
    sensor.sensor = true;
    sensor.layers.memberships = 2;
    world
        .insert_hull_body(
            id(1),
            RigidBody3d {
                kind: RigidBodyKind::Static,
                ..RigidBody3d::default()
            },
            &[],
            &[sensor],
        )
        .unwrap();
    assert!(ray(&world, 0.0).is_none());
    let filter = RaycastFilter3d {
        mask: 2,
        include_sensors: true,
        exclude: None,
    };
    assert!(
        world
            .raycast([0.0, 2.0, 0.0], [0.0, -1.0, 0.0], 4.0, filter)
            .unwrap()
            .is_some()
    );
    for filter in [
        RaycastFilter3d { mask: 1, ..filter },
        RaycastFilter3d {
            exclude: Some(id(1)),
            ..filter
        },
    ] {
        assert!(
            world
                .raycast([0.0, 2.0, 0.0], [0.0, -1.0, 0.0], 4.0, filter)
                .unwrap()
                .is_none()
        );
    }
    world
        .insert_body(
            id(2),
            RigidBody3d {
                position: [0.0; 3],
                ..RigidBody3d::default()
            },
            &[Collider3d::sphere(0.1)],
        )
        .unwrap();
    let dt = Duration::from_secs_f32(1.0 / 60.0);
    assert!(
        world
            .step(dt)
            .unwrap()
            .iter()
            .any(|event| event.kind == PhysicsEventKind::SensorEntered)
    );
    world.move_to(id(2), at([3.0, 0.0, 0.0])).unwrap();
    assert!(
        world
            .step(dt)
            .unwrap()
            .iter()
            .any(|event| event.kind == PhysicsEventKind::SensorExited)
    );
}

#[test]
fn rebuild_validation_ignores_registration_but_insertion_keeps_ownership() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let body = RigidBody3d::default();
    let pieces = [cube()];
    world.insert_hull_body(id(1), body, &[], &pieces).unwrap();
    world.validate_hull_body(id(1), body, &[], &pieces).unwrap();
    assert!(matches!(
        world.insert_hull_body(id(1), body, &[], &pieces),
        Err(PhysicsError::EntityAlreadyRegistered(_))
    ));
    let mut invalid = body;
    invalid.position[0] = f32::NAN;
    assert!(
        world
            .validate_hull_body(id(1), invalid, &[], &pieces)
            .is_err()
    );
    assert_eq!(ray(&world, 0.0).unwrap().entity, id(1));
}
