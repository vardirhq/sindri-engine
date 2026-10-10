//! Real queries, simulation and authoring share resolved collider scale.

use super::tests::{components, near3, spawn, step};
use super::*;
use sindri_core::{EntityData, Transform3D};
use sindri_physics::RaycastFilter3d;

fn resize(world: &mut World, entity: EntityId, scale: [f32; 3]) {
    world
        .get_mut(entity)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .scale = scale;
}

fn hit(physics: &ScenePhysics3d, point: [f32; 3]) -> Vec<EntityId> {
    physics
        .world()
        .overlap(
            ColliderShape3d::Sphere { radius: 0.01 },
            PhysicsPose3d {
                position: point,
                ..PhysicsPose3d::default()
            },
            RaycastFilter3d::default(),
        )
        .unwrap()
}

#[test]
fn parent_scale_resizes_box_offsets_and_editor_wireframes_once() {
    let mut world = World::default();
    let parent = world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            scale: [2.0, 3.0, 4.0],
            ..Transform3D::default()
        }),
        ..EntityData::default()
    });
    let piece = Collider3d {
        offset: [1.0, 0.0, 0.0],
        ..Collider3d::cuboid([0.5; 3])
    };
    let child = spawn(&mut world, [0.0; 3], None, piece);
    world.set_parent(child, Some(parent)).unwrap();
    resize(&mut world, child, [0.5, 1.0, 1.0]);
    let authored = world.get(child).unwrap().components.clone();
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    step(&mut physics, &mut world);
    assert_eq!(hit(&physics, [1.4, 1.4, 1.9]), vec![child]);
    assert!(hit(&physics, [1.6, 0.0, 0.0]).is_empty());
    let gizmos = crate::shape_gizmos(&world, &components());
    let collider = gizmos.iter().find(|gizmo| gizmo.entity == child).unwrap();
    assert!(
        collider
            .strokes
            .iter()
            .flat_map(|stroke| &stroke.points)
            .any(|point| point
                .iter()
                .zip([1.5, 1.5, 2.0])
                .all(|(a, b)| (a - b).abs() < 0.001))
    );
    assert_eq!(world.get(child).unwrap().components, authored);
}

#[test]
fn uniform_sphere_and_rotated_capsule_land_on_scaled_ground() {
    for piece in [
        Collider3d::sphere(0.25),
        Collider3d {
            rotation: [
                0.0,
                0.0,
                std::f32::consts::FRAC_1_SQRT_2,
                std::f32::consts::FRAC_1_SQRT_2,
            ],
            ..Collider3d::capsule(0.5, 0.25)
        },
    ] {
        let mut world = World::default();
        let ground = spawn(
            &mut world,
            [0.0; 3],
            None,
            Collider3d::cuboid([2.0, 0.25, 2.0]),
        );
        resize(&mut world, ground, [1.0, 2.0, 1.0]);
        let actor = spawn(
            &mut world,
            [0.0, 3.0, 0.0],
            Some(RigidBodyKind::Dynamic),
            piece,
        );
        resize(&mut world, actor, [2.0; 3]);
        let mut physics = ScenePhysics3d::new([0.0, -9.81, 0.0]).unwrap();
        for _ in 0..200 {
            step(&mut physics, &mut world);
        }
        assert!((world.world_transform(actor).unwrap().position[1] - 1.0).abs() < 0.06);
        near3(world.world_transform(actor).unwrap().scale, [2.0; 3]);
    }
}

#[test]
fn ancestor_resize_rebuilds_once_and_unchanged_steps_preserve_live_motion() {
    let mut world = World::default();
    let parent = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        ..EntityData::default()
    });
    let entity = spawn(
        &mut world,
        [0.0; 3],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.25),
    );
    world.set_parent(entity, Some(parent)).unwrap();
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    step(&mut physics, &mut world);
    physics
        .world_mut()
        .set_linear_velocity(entity, [1.0, 0.0, 0.0])
        .unwrap();
    resize(&mut world, parent, [2.0; 3]);
    step(&mut physics, &mut world);
    near3(physics.world().linear_velocity(entity).unwrap(), [0.0; 3]);
    assert_eq!(hit(&physics, [0.45, 0.0, 0.0]), vec![entity]);
    physics
        .world_mut()
        .set_linear_velocity(entity, [1.0, 0.0, 0.0])
        .unwrap();
    for _ in 0..10 {
        step(&mut physics, &mut world);
    }
    near3(
        physics.world().linear_velocity(entity).unwrap(),
        [1.0, 0.0, 0.0],
    );
    near3(
        world.world_transform(entity).unwrap().position,
        [0.1, 0.0, 0.0],
    );
}

#[test]
fn invalid_scale_batch_retains_gravity_removals_motion_and_geometry() {
    let mut world = World::default();
    let old = spawn(
        &mut world,
        [0.0; 3],
        Some(RigidBodyKind::Dynamic),
        Collider3d::sphere(0.25),
    );
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    step(&mut physics, &mut world);
    physics
        .world_mut()
        .set_linear_velocity(old, [1.0; 3])
        .unwrap();
    world.get_mut(old).unwrap().disabled = true;
    let bad = spawn(&mut world, [3.0; 3], None, Collider3d::sphere(0.25));
    let mut settings = EntityData::default();
    settings.components.insert(
        "sindri.physics3d.world".into(),
        serde_json::json!({"gravity": [0.0, -9.81, 0.0]}),
    );
    world.spawn(settings);
    for scale in [
        [0.0; 3],
        [-1.0; 3],
        [f32::NAN; 3],
        [f32::INFINITY; 3],
        [1.0, 2.0, 1.0],
    ] {
        resize(&mut world, bad, scale);
        assert!(
            matches!(physics.step(&mut world, &components(), Duration::from_millis(10)),
            Err(PhysicsSyncError::ColliderScale3d(entity, _)) if entity == bad)
        );
        assert!(!physics.world().contains(bad));
        assert!(physics.world().contains(old));
        near3(physics.world().linear_velocity(old).unwrap(), [1.0; 3]);
        near3(physics.world().gravity(), [0.0; 3]);
        assert_eq!(hit(&physics, [0.2, 0.0, 0.0]), vec![old]);
    }
}

#[test]
fn axis_permuted_boxes_scale_exactly_but_shear_and_overflow_fail() {
    let mut piece = Collider3d::cuboid([1.0, 2.0, 3.0]);
    piece.rotation = glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2).to_array();
    let scaled = Collider3dComponent(vec![piece])
        .scaled([2.0, 3.0, 4.0])
        .unwrap();
    let ColliderShape3d::Box { half_extents } = scaled[0].shape else {
        panic!("box expected")
    };
    near3(half_extents, [3.0, 4.0, 12.0]);
    piece.rotation = glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_4).to_array();
    assert!(
        Collider3dComponent(vec![piece])
            .scaled([2.0, 3.0, 4.0])
            .is_err()
    );
    piece.rotation = [0.0, 0.0, 0.0, 1.0];
    assert!(
        Collider3dComponent(vec![piece])
            .scaled([f32::MAX; 3])
            .is_err()
    );
    piece.offset = [2.0, 0.0, 0.0];
    assert!(
        Collider3dComponent(vec![piece])
            .scaled([f32::MAX; 3])
            .is_err()
    );
    let compound = Collider3dComponent(vec![
        Collider3d::cuboid([0.5; 3]),
        Collider3d::capsule(1.0, 0.5),
    ]);
    assert!(matches!(
        compound.scaled([2.0, 3.0, 4.0]),
        Err(crate::ColliderScaleError3d::Piece { index: 1, .. })
    ));
}
