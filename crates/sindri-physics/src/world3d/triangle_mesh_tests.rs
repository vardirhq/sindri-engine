use super::*;
use crate::{
    BodyControl3d, CharacterOptions3d, Collider3d, ColliderShape3d, CollisionLayers,
    GroundOptions3d, RaycastFilter3d, RigidBody3d, RigidBodyKind, TriangleMesh3d,
    TriangleMeshError3d,
};
use std::time::Duration;

fn id(value: u64) -> EntityId {
    EntityId::from_bits(value << 32)
}

fn at(position: [f32; 3]) -> PhysicsPose3d {
    PhysicsPose3d {
        position,
        ..PhysicsPose3d::default()
    }
}

fn floor() -> TriangleMeshCollider3d {
    TriangleMeshCollider3d::new(
        TriangleMesh3d::new(
            &[
                [-5.0, 0.0, -5.0],
                [-5.0, 0.0, 5.0],
                [5.0, 0.0, 5.0],
                [5.0, 0.0, -5.0],
            ],
            &[[0, 1, 2], [0, 2, 3]],
        )
        .unwrap(),
    )
}

fn ray(world: &PhysicsWorld3d, x: f32) -> Option<crate::RayHit3d> {
    world
        .raycast(
            [x, 4.0, 0.0],
            [0.0, -1.0, 0.0],
            10.0,
            RaycastFilter3d::default(),
        )
        .unwrap()
}

#[test]
fn geometry_rejects_bad_buffers_before_backend_construction() {
    let vertices = [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
    assert!(matches!(
        TriangleMesh3d::new(&[], &[[0, 1, 2]]),
        Err(TriangleMeshError3d::Empty)
    ));
    assert!(matches!(
        TriangleMesh3d::new(&vertices, &[]),
        Err(TriangleMeshError3d::Empty)
    ));
    for value in [f32::NAN, f32::INFINITY, 100_000_008.0] {
        let mut invalid = vertices;
        invalid[1][0] = value;
        assert!(matches!(
            TriangleMesh3d::new(&invalid, &[[0, 1, 2]]),
            Err(TriangleMeshError3d::Vertex(1))
        ));
    }
    assert!(matches!(
        TriangleMesh3d::new(&vertices, &[[0, 1, u32::MAX]]),
        Err(TriangleMeshError3d::Index(0))
    ));
    assert!(matches!(
        TriangleMesh3d::new(&vertices, &[[0, 1, 1]]),
        Err(TriangleMeshError3d::Area(0))
    ));
    let tiny = [[0.0; 3], [1.0e-20, 0.0, 0.0], [0.0, 0.0, 1.0e-20]];
    assert!(matches!(
        TriangleMesh3d::new(&tiny, &[[0, 1, 2]]),
        Err(TriangleMeshError3d::Area(0))
    ));
    let oversized = vec![[0.0; 3]; 1_000_001];
    assert!(matches!(
        TriangleMesh3d::new(&oversized, &[[0, 1, 2]]),
        Err(TriangleMeshError3d::Limit)
    ));
    let oversized = vec![[0, 1, 2]; 1_000_001];
    assert!(matches!(
        TriangleMesh3d::new(&vertices, &oversized),
        Err(TriangleMeshError3d::Limit)
    ));
}

#[test]
fn open_mesh_keeps_gaps_and_rays_hit_both_sides() {
    let geometry = TriangleMesh3d::new(
        &[
            [-3.0, 0.0, -1.0],
            [-3.0, 0.0, 1.0],
            [-1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [3.0, 0.0, 1.0],
            [3.0, 0.0, -1.0],
        ],
        &[[0, 1, 2], [3, 4, 5]],
    )
    .unwrap();
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .replace_static_mesh_group(
            id(1),
            1,
            at([0.0; 3]),
            &[TriangleMeshCollider3d::new(geometry)],
        )
        .unwrap();
    assert!(ray(&world, 0.0).is_none());
    for x in [-2.0, 2.0] {
        assert!(ray(&world, x).is_some());
        assert!(
            world
                .raycast(
                    [x, -2.0, 0.0],
                    [0.0, 1.0, 0.0],
                    4.0,
                    RaycastFilter3d::default()
                )
                .unwrap()
                .is_some()
        );
    }
    assert!(
        world
            .overlap(
                ColliderShape3d::Sphere { radius: 0.25 },
                at([0.0; 3]),
                RaycastFilter3d::default()
            )
            .unwrap()
            .is_empty()
    );
}

#[test]
fn meshes_share_geometry_and_update_all_queries_before_solving() {
    let piece = floor();
    let copied = piece.clone();
    assert!(std::sync::Arc::ptr_eq(
        &piece.geometry.shape.0,
        &copied.geometry.shape.0
    ));
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .replace_static_mesh_group(id(1), 7, at([0.0; 3]), &[piece])
        .unwrap();
    assert_eq!(ray(&world, 0.0).unwrap().entity, id(1));
    let hit = world
        .shape_cast(
            ColliderShape3d::Sphere { radius: 0.5 },
            at([0.0, 2.0, 0.0]),
            [0.0, -1.0, 0.0],
            4.0,
            RaycastFilter3d::default(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(hit.entity, id(1));
    assert!((hit.distance - 1.5).abs() < 0.001);
    assert_eq!(
        world
            .overlap(
                ColliderShape3d::Sphere { radius: 0.5 },
                at([0.0, 0.2, 0.0]),
                RaycastFilter3d::default()
            )
            .unwrap(),
        [id(1)]
    );
    let ground = world
        .probe_ground(
            ColliderShape3d::Sphere { radius: 0.5 },
            at([0.0, 0.52, 0.0]),
            GroundOptions3d::default(),
            RaycastFilter3d::default(),
        )
        .unwrap();
    assert!(ground.walkable && !ground.started_penetrating);
    assert_eq!(ground.hit.unwrap().entity, id(1));
    let movement = world
        .move_character(
            ColliderShape3d::Sphere { radius: 0.5 },
            at([0.0, 2.0, 0.0]),
            [0.0, -3.0, 0.0],
            CharacterOptions3d::default(),
            RaycastFilter3d::default(),
        )
        .unwrap();
    assert!(movement.translation[1] > -1.51 && movement.translation[1] < -1.45);
    assert!(
        movement
            .collisions
            .iter()
            .any(|hit| hit.hit.entity == id(1))
    );
    world.move_to(id(1), at([20.0, 1.0, 0.0])).unwrap();
    assert!(ray(&world, 0.0).is_none());
    assert!((ray(&world, 20.0).unwrap().point[1] - 1.0).abs() < 0.001);
    let mut copy = world.clone();
    copy.remove_static_group(id(1), 7).unwrap();
    assert!(ray(&copy, 20.0).is_none());
    assert!(ray(&world, 20.0).is_some());
}

#[test]
fn replacement_preserves_other_groups_and_primitive_handles() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .insert_static_collider(id(1), at([0.0; 3]), &[Collider3d::sphere(0.1)])
        .unwrap();
    let base = world.bodies[&id(1)].colliders[0];
    let body = world.bodies[&id(1)].body;
    world
        .replace_static_mesh_group(id(1), 20, at([0.0; 3]), &[floor()])
        .unwrap();
    let staying = world.bodies[&id(1)].groups[&20][0];
    world
        .replace_static_group(id(1), 10, at([0.0; 3]), &[Collider3d::sphere(0.2)])
        .unwrap();
    let old = world.bodies[&id(1)].groups[&10][0];
    let mut shifted = floor();
    shifted.pose = at([20.0, 1.0, 0.0]);
    world
        .replace_static_mesh_group(id(1), 10, at([0.0; 3]), &[shifted])
        .unwrap();
    assert_eq!(world.bodies[&id(1)].body, body);
    assert_eq!(world.bodies[&id(1)].colliders[0], base);
    assert_eq!(world.bodies[&id(1)].groups[&20], [staying]);
    assert!(world.backend.colliders.get(old).is_none());
    assert!(ray(&world, 20.0).is_some());
    world
        .replace_static_mesh_group(id(1), 10, at([0.0; 3]), &[])
        .unwrap();
    assert!(ray(&world, 20.0).is_none());
    assert_eq!(world.bodies[&id(1)].colliders, [base, staying]);
    world.remove(id(1));
    assert!(world.backend.colliders.get(staying).is_none());
    world
        .replace_static_mesh_group(id(2), 1, at([0.0; 3]), &[])
        .unwrap();
    assert!(!world.contains(id(2)));
}

#[test]
fn invalid_late_piece_pose_or_owner_keeps_existing_geometry() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .replace_static_mesh_group(id(1), 7, at([0.0; 3]), &[floor()])
        .unwrap();
    let handle = world.bodies[&id(1)].groups[&7][0];
    let mut invalid = floor();
    invalid.material.restitution = 2.0;
    assert!(matches!(
        world.replace_static_mesh_group(id(1), 7, at([20.0; 3]), &[floor(), invalid]),
        Err(PhysicsError::ColliderPiece { index: 1, .. })
    ));
    assert_eq!(world.bodies[&id(1)].groups[&7], [handle]);
    assert!(ray(&world, 0.0).is_some());
    let mut overflow = floor();
    overflow.pose = at([f32::MAX, 0.0, 0.0]);
    assert!(
        world
            .replace_static_mesh_group(id(1), 7, at([f32::MAX, 0.0, 0.0]), &[overflow])
            .is_err()
    );
    assert_eq!(world.bodies[&id(1)].groups[&7], [handle]);
    for pose in [
        at([f32::NAN, 0.0, 0.0]),
        PhysicsPose3d {
            rotation: [0.0; 4],
            ..at([0.0; 3])
        },
    ] {
        assert!(
            world
                .replace_static_mesh_group(id(1), 7, pose, &[floor()])
                .is_err()
        );
        let mut invalid = floor();
        invalid.pose = pose;
        assert!(
            world
                .replace_static_mesh_group(id(1), 7, at([0.0; 3]), &[invalid])
                .is_err()
        );
        assert_eq!(world.bodies[&id(1)].groups[&7], [handle]);
    }
    world
        .insert_body(id(2), RigidBody3d::default(), &[Collider3d::sphere(0.5)])
        .unwrap();
    assert!(matches!(
        world.replace_static_mesh_group(id(2), 7, at([0.0; 3]), &[floor()]),
        Err(PhysicsError::WrongBodyKind(..))
    ));
    world
        .remember_control(
            id(3),
            RigidBodyKind::Dynamic,
            BodyControl3d::Impulse([1.0, 0.0, 0.0]),
        )
        .unwrap();
    assert!(
        world
            .replace_static_mesh_group(id(3), 7, at([0.0; 3]), &[floor()])
            .is_err()
    );
    assert!(!world.contains(id(3)));
    assert!(world.pending_controls.contains_key(&id(3)));
}

#[test]
fn filtering_and_local_rotation_follow_existing_collider_rules() {
    let mut piece = floor();
    piece.sensor = true;
    piece.layers = CollisionLayers {
        memberships: 2,
        filter: u32::MAX,
    };
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    world
        .replace_static_mesh_group(id(1), 1, at([0.0; 3]), &[piece.clone()])
        .unwrap();
    assert!(ray(&world, 0.0).is_none());
    let filter = RaycastFilter3d {
        include_sensors: true,
        mask: 2,
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
    piece.sensor = false;
    let half = std::f32::consts::FRAC_PI_4;
    piece.pose.rotation = [half.sin(), 0.0, 0.0, half.cos()];
    world
        .replace_static_mesh_group(id(1), 1, at([0.0; 3]), &[piece])
        .unwrap();
    assert!(
        world
            .raycast(
                [0.0, 0.0, 2.0],
                [0.0, 0.0, -1.0],
                4.0,
                RaycastFilter3d::default()
            )
            .unwrap()
            .is_some()
    );
}

#[test]
fn dynamic_body_lands_on_static_triangles_and_replays() {
    let mut world = PhysicsWorld3d::new([0.0, -9.81, 0.0]).unwrap();
    world
        .replace_static_mesh_group(id(1), 1, at([0.0; 3]), &[floor()])
        .unwrap();
    world
        .insert_body(
            id(2),
            RigidBody3d {
                position: [0.0, 2.0, 0.0],
                ..RigidBody3d::default()
            },
            &[Collider3d::sphere(0.5)],
        )
        .unwrap();
    let mut copy = world.clone();
    let mut touched = false;
    for _ in 0..180 {
        let events = world.step(Duration::from_secs_f32(1.0 / 60.0)).unwrap();
        assert_eq!(
            events,
            copy.step(Duration::from_secs_f32(1.0 / 60.0)).unwrap()
        );
        touched |= events
            .iter()
            .any(|event| event.first == id(1) && event.second == id(2));
    }
    let pose = world.pose(id(2)).unwrap();
    assert!((pose.position[1] - 0.5).abs() < 0.02, "{pose:?}");
    assert_eq!(pose, copy.pose(id(2)).unwrap());
    assert!(touched);
}
