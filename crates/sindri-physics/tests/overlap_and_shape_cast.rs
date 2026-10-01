//! Area checks and swept shapes against the pieces a world holds.

use sindri_core::EntityId;
use sindri_physics::{
    Collider2d, ColliderShape2d, CollisionLayers, PhysicsPose2d, PhysicsWorld2d, RaycastFilter2d,
    RigidBody2d, RigidBodyKind,
};

fn entity(index: u32) -> EntityId {
    EntityId::from_bits(u64::from(index) << 32)
}

fn at(x: f32, y: f32) -> PhysicsPose2d {
    PhysicsPose2d {
        position: [x, y],
        rotation: 0.0,
    }
}

fn wall(world: &mut PhysicsWorld2d, id: u32, x: f32, y: f32, collider: Collider2d) {
    world
        .insert_body(
            entity(id),
            RigidBody2d {
                kind: RigidBodyKind::Static,
                pose: at(x, y),
                ..RigidBody2d::default()
            },
            &[collider],
        )
        .expect("body");
}

const CIRCLE: ColliderShape2d = ColliderShape2d::Circle { radius: 0.5 };

#[test]
fn an_area_finds_each_entity_it_overlaps_once() {
    let mut world = PhysicsWorld2d::new([0.0, 0.0]).unwrap();
    wall(&mut world, 1, 0.0, 0.0, Collider2d::rectangle([1.0, 1.0]));
    wall(&mut world, 2, 1.8, 0.0, Collider2d::circle(0.4));
    wall(&mut world, 3, 5.0, 0.0, Collider2d::circle(0.4));
    let mut sensor = Collider2d::circle(0.4);
    sensor.sensor = true;
    wall(&mut world, 4, 0.0, 1.2, sensor);

    let found = world
        .overlap(CIRCLE, at(1.2, 0.0), RaycastFilter2d::default())
        .unwrap();
    assert_eq!(found, [entity(1), entity(2)]);

    let with_sensors = RaycastFilter2d {
        include_sensors: true,
        exclude: Some(entity(2)),
        ..RaycastFilter2d::default()
    };
    let found = world.overlap(CIRCLE, at(0.5, 1.0), with_sensors).unwrap();
    assert_eq!(found, [entity(1), entity(4)]);

    assert!(
        world
            .overlap(CIRCLE, at(3.4, 0.0), RaycastFilter2d::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_mask_picks_the_layers_an_area_sees() {
    let mut world = PhysicsWorld2d::new([0.0, 0.0]).unwrap();
    let mut enemy = Collider2d::circle(0.5);
    enemy.layers = CollisionLayers {
        memberships: 0b10,
        filter: u32::MAX,
    };
    let mut ground = Collider2d::circle(0.5);
    ground.layers = CollisionLayers {
        memberships: 0b01,
        filter: u32::MAX,
    };
    wall(&mut world, 1, 0.0, 0.0, ground);
    wall(&mut world, 2, 0.2, 0.0, enemy);
    let enemies = RaycastFilter2d {
        mask: 0b10,
        ..RaycastFilter2d::default()
    };
    assert_eq!(
        world.overlap(CIRCLE, at(0.0, 0.0), enemies).unwrap(),
        [entity(2)]
    );
}

#[test]
fn a_swept_shape_stops_at_the_first_surface_its_edge_reaches() {
    let mut world = PhysicsWorld2d::new([0.0, 0.0]).unwrap();
    wall(&mut world, 1, 5.0, 0.0, Collider2d::rectangle([1.0, 3.0]));
    wall(&mut world, 2, 9.0, 0.0, Collider2d::rectangle([1.0, 3.0]));

    let hit = world
        .shape_cast(
            CIRCLE,
            at(0.0, 0.0),
            [2.0, 0.0],
            10.0,
            RaycastFilter2d::default(),
        )
        .unwrap()
        .expect("a hit");
    assert_eq!(hit.entity, entity(1));
    // The circle's edge, half a unit ahead of its centre, meets the wall's
    // face at x = 4.
    assert!((hit.distance - 3.5).abs() < 1.0e-3, "{hit:?}");
    assert!((hit.point[0] - 4.0).abs() < 1.0e-3, "{hit:?}");
    assert!((hit.normal[0] + 1.0).abs() < 1.0e-3, "{hit:?}");

    // A ray down the centre line slips past a ledge the circle cannot.
    wall(&mut world, 3, 2.0, 0.6, Collider2d::rectangle([0.2, 0.2]));
    let hit = world
        .shape_cast(
            CIRCLE,
            at(0.0, 0.0),
            [1.0, 0.0],
            10.0,
            RaycastFilter2d::default(),
        )
        .unwrap()
        .expect("a hit");
    assert_eq!(hit.entity, entity(3));
    assert!(
        world
            .raycast([0.0, 0.0], [1.0, 0.0], 3.0, RaycastFilter2d::default())
            .unwrap()
            .is_none()
    );

    // Too short to arrive, nothing; starting inside, a zero-distance hit.
    assert!(
        world
            .shape_cast(
                CIRCLE,
                at(0.0, -2.0),
                [1.0, 0.0],
                3.0,
                RaycastFilter2d::default()
            )
            .unwrap()
            .is_none()
    );
    let inside = world
        .shape_cast(
            CIRCLE,
            at(5.0, 0.0),
            [1.0, 0.0],
            1.0,
            RaycastFilter2d::default(),
        )
        .unwrap()
        .expect("starting inside");
    assert!(inside.distance.abs() < f32::EPSILON);
    assert!(inside.normal.iter().all(|n| n.abs() < f32::EPSILON));
}

#[test]
fn bad_queries_are_refused() {
    let world = PhysicsWorld2d::new([0.0, 0.0]).unwrap();
    let filter = RaycastFilter2d::default();
    assert!(
        world
            .overlap(
                ColliderShape2d::Circle { radius: 0.0 },
                at(0.0, 0.0),
                filter
            )
            .is_err()
    );
    assert!(world.overlap(CIRCLE, at(f32::NAN, 0.0), filter).is_err());
    assert!(
        world
            .shape_cast(CIRCLE, at(0.0, 0.0), [0.0, 0.0], 1.0, filter)
            .is_err()
    );
    assert!(
        world
            .shape_cast(CIRCLE, at(0.0, 0.0), [1.0, 0.0], -1.0, filter)
            .is_err()
    );
}
