//! Public query geometry, filters, determinism and lifecycle.
use sindri_core::EntityId;
use sindri_physics::{
    Collider2d, ColliderShape2d, CollisionLayers, PhysicsError, PhysicsPose2d, PhysicsWorld2d,
    RaycastFilter2d, RigidBody2d, RigidBodyKind,
};
use std::time::Duration;

fn id(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}
fn at(x: f32, y: f32) -> PhysicsPose2d {
    PhysicsPose2d {
        position: [x, y],
        rotation: 0.0,
    }
}
fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}");
}

#[test]
fn closest_hit_normalizes_direction_and_includes_the_distance_boundary() {
    let mut world = PhysicsWorld2d::new([0.0, 0.0]).unwrap();
    world
        .insert_static_collider(id(8), at(5.0, 0.0), &[Collider2d::circle(1.0)])
        .unwrap();
    world
        .insert_static_collider(id(2), at(3.0, 0.0), &[Collider2d::rectangle([0.5, 1.0])])
        .unwrap();
    let hit = world
        .raycast([0.0; 2], [20.0, 0.0], 2.5, RaycastFilter2d::default())
        .unwrap()
        .unwrap();
    assert_eq!(hit.entity, id(2));
    near(hit.distance, 2.5);
    near(hit.point[0], 2.5);
    near(hit.point[1], 0.0);
    near(hit.normal[0], -1.0);
    near(hit.normal[1], 0.0);
    assert!(
        world
            .raycast([0.0; 2], [1.0, 0.0], 2.49, RaycastFilter2d::default())
            .unwrap()
            .is_none()
    );
    assert!(
        world
            .raycast([0.0; 2], [-1.0, 0.0], 10.0, RaycastFilter2d::default())
            .unwrap()
            .is_none()
    );
}

#[test]
fn masks_sensors_exclusion_and_predicates_filter_before_choosing_closest() {
    let mut world = PhysicsWorld2d::new([0.0, 0.0]).unwrap();
    let mut sensor = Collider2d::circle(0.5);
    sensor.sensor = true;
    sensor.layers = CollisionLayers::new(2, 0);
    let mut solid = Collider2d::circle(0.5);
    solid.layers = CollisionLayers::new(1, 0);
    world
        .insert_static_collider(id(1), at(2.0, 0.0), &[sensor])
        .unwrap();
    world
        .insert_static_collider(id(2), at(4.0, 0.0), &[solid])
        .unwrap();
    let query = |filter| world.raycast([0.0; 2], [1.0, 0.0], 10.0, filter).unwrap();
    assert_eq!(query(RaycastFilter2d::default()).unwrap().entity, id(2));
    let all = RaycastFilter2d {
        include_sensors: true,
        ..RaycastFilter2d::default()
    };
    assert_eq!(query(all).unwrap().entity, id(1));
    assert_eq!(
        query(RaycastFilter2d { mask: 1, ..all }).unwrap().entity,
        id(2)
    );
    assert_eq!(
        query(RaycastFilter2d {
            exclude: Some(id(1)),
            ..all
        })
        .unwrap()
        .entity,
        id(2)
    );
    assert!(query(RaycastFilter2d { mask: 0, ..all }).is_none());
    assert_eq!(
        world
            .raycast_where([0.0; 2], [1.0, 0.0], 10.0, all, |e| e != id(1))
            .unwrap()
            .unwrap()
            .entity,
        id(2)
    );
}

#[test]
fn inside_and_zero_length_segments_have_a_defined_zero_normal() {
    let mut world = PhysicsWorld2d::new([0.0, 0.0]).unwrap();
    world
        .insert_static_collider(id(1), at(0.0, 0.0), &[Collider2d::circle(1.0)])
        .unwrap();
    let hit = world
        .raycast([0.0; 2], [1.0, 0.0], 0.0, RaycastFilter2d::default())
        .unwrap()
        .unwrap();
    near(hit.distance, 0.0);
    near(hit.normal[0], 0.0);
    near(hit.normal[1], 0.0);
    assert!(
        world
            .raycast([2.0, 0.0], [1.0, 0.0], 0.0, RaycastFilter2d::default())
            .unwrap()
            .is_none()
    );
}

#[test]
fn compound_offsets_rotations_capsules_and_ties_are_stable() {
    let mut world = PhysicsWorld2d::new([0.0, 0.0]).unwrap();
    let mut piece = Collider2d::rectangle([1.0, 0.25]);
    piece.offset = [1.0, 0.0];
    piece.rotation = std::f32::consts::FRAC_PI_2;
    world
        .insert_static_collider(id(9), at(3.0, 0.0), &[Collider2d::circle(0.1), piece])
        .unwrap();
    let hit = world
        .raycast([4.0, 2.0], [0.0, -1.0], 5.0, RaycastFilter2d::default())
        .unwrap()
        .unwrap();
    near(hit.point[1], 1.0);
    near(hit.normal[1], 1.0);
    let mut capsule = Collider2d::circle(0.5);
    capsule.shape = ColliderShape2d::Capsule {
        half_height: 0.5,
        radius: 0.5,
    };
    world
        .insert_static_collider(id(2), at(6.0, 0.0), &[capsule])
        .unwrap();
    let capsule_hit = world
        .raycast([6.0, 2.0], [0.0, -1.0], 5.0, RaycastFilter2d::default())
        .unwrap()
        .unwrap();
    assert_eq!(capsule_hit.entity, id(2));
    near(capsule_hit.distance, 1.0);
    world
        .insert_static_collider(id(3), at(3.0, 0.0), &[Collider2d::circle(0.1), piece])
        .unwrap();
    for _ in 0..10 {
        let hit = world
            .raycast([4.0, 2.0], [0.0, -1.0], 5.0, RaycastFilter2d::default())
            .unwrap()
            .unwrap();
        assert_eq!(hit.entity, id(3));
        near(hit.distance, 1.0);
    }
}

#[test]
fn queries_track_move_step_remove_and_reused_handles_with_current_geometry() {
    let mut world = PhysicsWorld2d::new([0.0, 0.0]).unwrap();
    world
        .insert_body(
            id(1),
            RigidBody2d {
                kind: RigidBodyKind::KinematicVelocity,
                pose: at(2.0, 0.0),
                linear_velocity: [1.0, 0.0],
                ..RigidBody2d::default()
            },
            &[Collider2d::circle(0.5)],
        )
        .unwrap();
    world.move_to(id(1), at(3.0, 0.0)).unwrap();
    let query = |w: &PhysicsWorld2d| {
        w.raycast([0.0; 2], [1.0, 0.0], 10.0, RaycastFilter2d::default())
            .unwrap()
    };
    near(query(&world).unwrap().distance, 2.5);
    world.step(Duration::from_millis(100)).unwrap();
    near(query(&world).unwrap().distance, 2.6);
    world.remove(id(1));
    assert!(query(&world).is_none());
    let reused = EntityId::from_bits(id(1).to_bits() | 1);
    world
        .insert_static_collider(reused, at(2.0, 0.0), &[Collider2d::circle(0.5)])
        .unwrap();
    assert_eq!(query(&world).unwrap().entity, reused);
}

#[test]
fn invalid_values_fail_while_extreme_finite_directions_normalize() {
    let mut world = PhysicsWorld2d::new([0.0, 0.0]).unwrap();
    world
        .insert_static_collider(id(1), at(2.0, 0.0), &[Collider2d::circle(0.5)])
        .unwrap();
    let filter = RaycastFilter2d::default();
    for direction in [[0.0; 2], [f32::NAN, 1.0], [1.0, f32::INFINITY]] {
        assert!(world.raycast([0.0; 2], direction, 5.0, filter).is_err());
    }
    for distance in [-1.0, f32::NAN, f32::INFINITY] {
        assert!(
            world
                .raycast([0.0; 2], [1.0, 0.0], distance, filter)
                .is_err()
        );
    }
    assert!(matches!(
        world.raycast([f32::NAN, 0.0], [1.0, 0.0], 1.0, filter),
        Err(PhysicsError::NonFinite(_))
    ));
    for x in [f32::MAX, f32::from_bits(1)] {
        near(
            world
                .raycast([0.0; 2], [x, 0.0], 5.0, filter)
                .unwrap()
                .unwrap()
                .distance,
            1.5,
        );
    }
}
