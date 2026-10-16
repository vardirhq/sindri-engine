use super::*;
use crate::{Character3dComponent, Collider3dComponent, SceneExtractor};
use serde_json::json;
use sindri_core::{EntityData, EntityId, SceneComponent, Transform3D};
use sindri_physics::Collider3d;

fn actor(world: &mut World, piece: Collider3d, transform: Transform3D) -> EntityId {
    world.spawn(EntityData {
        transform_3d: Some(transform),
        components: [
            (
                Collider3dComponent::TYPE_NAME.into(),
                json!({"pieces": [piece]}),
            ),
            (
                Character3dComponent::TYPE_NAME.into(),
                json!({"step_height": 0.25, "snap_distance": 0.5}),
            ),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    })
}

fn near(actual: [f32; 3], expected: [f32; 3]) {
    assert!(
        actual
            .into_iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() < 1.0e-4),
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn scaled_offset_box_has_world_space_foot_step_and_snap_rings() {
    let extractor = SceneExtractor::new().unwrap();
    let mut world = World::default();
    let mut piece = Collider3d::cuboid([0.5, 1.0, 0.25]);
    piece.offset = [1.0, 0.0, 0.0];
    let entity = actor(
        &mut world,
        piece,
        Transform3D {
            position: [2.0, 3.0, 4.0],
            scale: [2.0, 2.0, 2.0],
            ..Transform3D::default()
        },
    );
    let gizmos = characters(&world, extractor.components());
    assert_eq!(gizmos.len(), 1);
    assert_eq!(gizmos[0].entity, entity);
    near(gizmos[0].marks[0], [4.0, 1.0, 4.0]);
    assert_eq!(gizmos[0].strokes.len(), 7);
    for point in &gizmos[0].strokes[5].points {
        assert!(
            (point[1] - 1.25).abs() < 1.0e-4,
            "step stays in world units"
        );
    }
    for point in &gizmos[0].strokes[6].points {
        assert!((point[1] - 0.5).abs() < 1.0e-4, "snap stays in world units");
    }
}

#[test]
fn parent_rotation_and_world_up_use_the_exact_capsule_projection() {
    let extractor = SceneExtractor::new().unwrap();
    let mut world = World::default();
    let parent = world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [10.0, 0.0, 0.0],
            scale: [2.0; 3],
            rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2).to_array(),
            ..Transform3D::default()
        }),
        ..EntityData::default()
    });
    let mut piece = Collider3d::cuboid([0.5; 3]);
    piece.shape = ColliderShape3d::Capsule {
        half_height: 1.0,
        radius: 0.5,
    };
    let entity = actor(
        &mut world,
        piece,
        Transform3D {
            position: [1.0, 0.0, 0.0],
            ..Transform3D::default()
        },
    );
    world.get_mut(entity).unwrap().parent = Some(parent);
    let gizmos = characters(&world, extractor.components());
    // The capsule lies along X. Its extent along world Y is only its radius.
    near(gizmos[0].marks[0], [10.0, 1.0, 0.0]);
    world
        .get_mut(entity)
        .unwrap()
        .components
        .get_mut(Character3dComponent::TYPE_NAME)
        .unwrap()["up"] = json!([1.0, 0.0, 0.0]);
    let gizmos = characters(&world, extractor.components());
    near(gizmos[0].marks[0], [7.0, 2.0, 0.0]);
    assert!(
        gizmos[0].strokes[5]
            .points
            .iter()
            .all(|p| (p[0] - 7.25).abs() < 1.0e-4)
    );
}

#[test]
fn sensors_are_ignored_and_unsupported_controller_geometry_is_not_drawn() {
    let extractor = SceneExtractor::new().unwrap();
    let mut world = World::default();
    let piece = Collider3d::cuboid([0.5; 3]);
    let entity = actor(&mut world, piece, Transform3D::default());
    let mut sensor = Collider3d::cuboid([10.0; 3]);
    sensor.sensor = true;
    *world
        .get_mut(entity)
        .unwrap()
        .components
        .get_mut(Collider3dComponent::TYPE_NAME)
        .unwrap() = json!({"pieces": [sensor, piece]});
    let gizmos = characters(&world, extractor.components());
    near(gizmos[0].marks[0], [0.0, -0.5, 0.0]);
    *world
        .get_mut(entity)
        .unwrap()
        .components
        .get_mut(Collider3dComponent::TYPE_NAME)
        .unwrap() = json!({"pieces": [piece, piece]});
    assert!(characters(&world, extractor.components()).is_empty());
    let mut sphere = piece;
    sphere.shape = ColliderShape3d::Sphere { radius: 0.5 };
    *world
        .get_mut(entity)
        .unwrap()
        .components
        .get_mut(Collider3dComponent::TYPE_NAME)
        .unwrap() = json!({"pieces": [sphere]});
    world
        .get_mut(entity)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .scale = [1.0, 2.0, 1.0];
    assert!(characters(&world, extractor.components()).is_empty());
    world
        .get_mut(entity)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .scale = [2.0; 3];
    near(
        characters(&world, extractor.components())[0].marks[0],
        [0.0, -1.0, 0.0],
    );
    world
        .get_mut(entity)
        .unwrap()
        .components
        .get_mut(Character3dComponent::TYPE_NAME)
        .unwrap()["skin"] = json!(-1.0);
    assert!(characters(&world, extractor.components()).is_empty());
}

#[test]
fn disabled_controllers_and_zero_reaches_do_not_leave_stale_guides() {
    let extractor = SceneExtractor::new().unwrap();
    let mut world = World::default();
    let entity = actor(
        &mut world,
        Collider3d::cuboid([0.5; 3]),
        Transform3D::default(),
    );
    *world
        .get_mut(entity)
        .unwrap()
        .components
        .get_mut(Character3dComponent::TYPE_NAME)
        .unwrap() = json!({});
    assert_eq!(
        characters(&world, extractor.components())[0].strokes.len(),
        5
    );
    world.get_mut(entity).unwrap().disabled = true;
    assert!(characters(&world, extractor.components()).is_empty());
}
