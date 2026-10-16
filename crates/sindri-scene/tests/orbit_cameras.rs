//! Orbit geometry, filtering and checked edits through the real scene seam.

use serde_json::json;
use sindri_core::{EntityData, EntityId, SceneComponent, SceneEntityId, Transform3D, World};
use sindri_physics::{Collider3d, CollisionLayers};
use sindri_scene::{
    CameraComponent, CameraOrbitComponent, SceneExtractor, ScenePhysics3d, clear_camera_orbit,
    set_camera_orbit, set_camera_orbit_collision, set_camera_orbit_offset,
    set_camera_orbit_smoothing, set_camera_perspective_fov, update_orbit_cameras,
};
use std::time::Duration;

fn spatial(world: &mut World, name: &str, position: [f32; 3]) -> EntityId {
    world.spawn(EntityData {
        source_id: Some(SceneEntityId::new(name).unwrap()),
        transform_3d: Some(Transform3D {
            position,
            ..Transform3D::default()
        }),
        ..EntityData::default()
    })
}
fn setup() -> (World, EntityId, EntityId) {
    let mut world = World::default();
    let target = spatial(&mut world, "target", [0.0; 3]);
    let camera = spatial(&mut world, "camera", [0.0, 0.0, 6.0]);
    world.get_mut(camera).unwrap().components.insert(CameraComponent::TYPE_NAME.to_owned(),
        json!({"projection":"perspective", "vertical_fov_degrees":60.0,"near":0.1,"far":100.0, "future":"retained"}));
    assert!(set_camera_orbit(&mut world, camera, target, 0.0, 0.0, 6.0));
    assert!(set_camera_orbit_smoothing(&mut world, camera, 0.0));
    (world, camera, target)
}
fn physics(world: &mut World) -> ScenePhysics3d {
    let mut physics = ScenePhysics3d::new([0.0; 3]).unwrap();
    physics
        .step(
            world,
            SceneExtractor::new().unwrap().components(),
            Duration::from_secs_f32(1.0 / 60.0),
        )
        .unwrap();
    physics
}
fn blocker(world: &mut World, name: &str, z: f32, sensor: bool, layer: u32) -> EntityId {
    let id = spatial(world, name, [0.0, 0.0, z]);
    let collider = Collider3d {
        sensor,
        layers: CollisionLayers {
            memberships: layer,
            filter: u32::MAX,
        },
        ..Collider3d::cuboid([3.0, 3.0, 0.5])
    };
    world.get_mut(id).unwrap().components.insert(
        "sindri.physics3d.collider".to_owned(),
        serde_json::to_value(collider).unwrap(),
    );
    id
}
fn assert_near(actual: [f32; 3], expected: [f32; 3]) {
    assert!(
        actual
            .into_iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() < 1.0e-4),
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn obstruction_pulls_in_immediately_and_release_smooths_out() {
    let (mut world, camera, _) = setup();
    let wall = blocker(&mut world, "wall", 3.0, false, 1);
    let physics = physics(&mut world);
    assert!(set_camera_orbit_smoothing(&mut world, camera, 8.0));
    assert!(update_orbit_cameras(&mut world, physics.world(), 1.0 / 60.0).is_empty());
    assert_near(
        world.world_transform(camera).unwrap().position,
        [0.0, 0.0, 2.3],
    );
    world.get_mut(wall).unwrap().disabled = true;
    assert!(update_orbit_cameras(&mut world, physics.world(), 1.0 / 60.0).is_empty());
    let released = world.world_transform(camera).unwrap().position[2];
    assert!(released > 2.3 && released < 6.0);
    for _ in 0..100 {
        assert!(update_orbit_cameras(&mut world, physics.world(), 1.0 / 60.0).is_empty());
    }
    assert!((world.world_transform(camera).unwrap().position[2] - 6.0).abs() < 1.0e-4);
}

#[test]
fn sensors_layers_target_and_camera_are_filtered() {
    let (mut world, camera, target) = setup();
    blocker(&mut world, "sensor", 1.0, true, 1);
    blocker(&mut world, "other-layer", 2.0, false, 2);
    blocker(&mut world, "wall", 3.0, false, 1);
    // Both ends carry solid geometry; neither is an obstruction to its own camera.
    let shape = serde_json::to_value(Collider3d {
        layers: CollisionLayers {
            memberships: 1,
            filter: u32::MAX,
        },
        ..Collider3d::sphere(0.5)
    })
    .unwrap();
    for id in [camera, target] {
        world
            .get_mut(id)
            .unwrap()
            .components
            .insert("sindri.physics3d.collider".to_owned(), shape.clone());
    }
    let physics = physics(&mut world);
    assert!(set_camera_orbit_collision(&mut world, camera, 1, 0.2));
    assert!(update_orbit_cameras(&mut world, physics.world(), 1.0 / 60.0).is_empty());
    assert_near(
        world.world_transform(camera).unwrap().position,
        [0.0, 0.0, 2.3],
    );
    assert!(set_camera_orbit_collision(&mut world, camera, 0, 0.2));
    assert!(update_orbit_cameras(&mut world, physics.world(), 1.0 / 60.0).is_empty());
    assert_near(
        world.world_transform(camera).unwrap().position,
        [0.0, 0.0, 6.0],
    );
}

#[test]
fn parented_camera_tracks_parented_target_in_world_space() {
    let (mut world, camera, target) = setup();
    let parent = spatial(&mut world, "parent", [5.0, 3.0, -2.0]);
    let pose = world
        .get_mut(parent)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap();
    pose.scale = [2.0, 3.0, 4.0];
    pose.set_yaw_pitch_roll_radians([0.8, 0.2, 0.1]);
    world.set_parent(target, Some(parent)).unwrap();
    world.set_parent(camera, Some(parent)).unwrap();
    let scale = [0.5, 0.6, 0.7];
    world
        .get_mut(camera)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .scale = scale;
    assert!(set_camera_orbit_offset(&mut world, camera, [0.0, 1.0, 0.0]));
    assert!(set_camera_orbit(
        &mut world,
        camera,
        target,
        std::f32::consts::FRAC_PI_2,
        0.0,
        6.0
    ));
    let physics = physics(&mut world);
    assert!(update_orbit_cameras(&mut world, physics.world(), 1.0 / 60.0).is_empty());
    let placed = world.world_transform(camera).unwrap();
    assert_near(placed.position, [11.0, 4.0, -2.0]);
    assert_near(placed.forward(), [-1.0, 0.0, 0.0]);
    assert_near(
        world.get(camera).unwrap().transform_3d.unwrap().scale,
        scale,
    );
}

#[test]
fn invalid_edits_and_invalid_authored_orbits_are_atomic() {
    let (mut world, camera, target) = setup();
    let before = world.get(camera).unwrap().clone();
    for (yaw, pitch, distance) in [
        (f32::NAN, 0.0, 6.0),
        (0.0, std::f32::consts::FRAC_PI_2, 6.0),
        (0.0, 0.0, 0.0),
    ] {
        assert!(!set_camera_orbit(
            &mut world, camera, target, yaw, pitch, distance
        ));
        assert_eq!(world.get(camera).unwrap(), &before);
    }
    assert!(!set_camera_orbit(&mut world, camera, camera, 0.0, 0.0, 6.0));
    assert!(!set_camera_orbit_collision(&mut world, camera, 1, 6.0));
    assert!(!set_camera_orbit_offset(
        &mut world,
        camera,
        [f32::INFINITY, 0.0, 0.0]
    ));
    world
        .get_mut(camera)
        .unwrap()
        .components
        .get_mut(CameraOrbitComponent::TYPE_NAME)
        .unwrap()["distance"] = json!(-1);
    let physics = physics(&mut world);
    assert_eq!(
        update_orbit_cameras(&mut world, physics.world(), 1.0 / 60.0).len(),
        1
    );
    assert_eq!(world.get(camera).unwrap().transform_3d, before.transform_3d);
}

#[test]
fn a_locked_camera_and_a_singular_parent_keep_their_pose() {
    let (mut world, camera, target) = setup();
    world
        .get_mut(camera)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .z_locked = true;
    assert!(set_camera_orbit(&mut world, camera, target, 1.0, 0.0, 6.0));
    let before = world.get(camera).unwrap().transform_3d;
    let physics = physics(&mut world);
    assert_eq!(
        update_orbit_cameras(&mut world, physics.world(), 1.0 / 60.0).len(),
        1
    );
    assert_eq!(world.get(camera).unwrap().transform_3d, before);
    let parent = spatial(&mut world, "singular", [0.0; 3]);
    world
        .get_mut(parent)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .scale = [0.0, 1.0, 1.0];
    world.set_parent(camera, Some(parent)).unwrap();
    assert_eq!(
        update_orbit_cameras(&mut world, physics.world(), 1.0 / 60.0).len(),
        1
    );
    assert_eq!(world.get(camera).unwrap().transform_3d, before);
}

#[test]
fn projection_and_orbit_controls_preserve_unknown_fields() {
    let (mut world, camera, target) = setup();
    let payload = world
        .get_mut(camera)
        .unwrap()
        .components
        .get_mut(CameraOrbitComponent::TYPE_NAME)
        .unwrap();
    payload["future"] = json!({"setting":42});
    assert!(set_camera_orbit(&mut world, camera, target, 0.5, 0.4, 8.0));
    assert_eq!(
        world.get(camera).unwrap().components[CameraOrbitComponent::TYPE_NAME]["future"]["setting"],
        42
    );
    assert!(set_camera_perspective_fov(&mut world, camera, 70.0));
    let payload = world.get(camera).unwrap().components[CameraComponent::TYPE_NAME].clone();
    assert_eq!(payload["future"], "retained");
    assert!((payload["near"].as_f64().unwrap() - 0.1).abs() < 1.0e-6);
    for fov in [0.0, 180.0, f32::NAN] {
        assert!(!set_camera_perspective_fov(&mut world, camera, fov));
        assert_eq!(
            world.get(camera).unwrap().components[CameraComponent::TYPE_NAME],
            payload
        );
    }
    let before = world.get(camera).unwrap().transform_3d;
    assert!(clear_camera_orbit(&mut world, camera));
    assert_eq!(world.get(camera).unwrap().transform_3d, before);
}

#[test]
fn orbit_suspends_2d_behavior_and_resume_does_not_subtract_a_stale_shake() {
    use sindri_scene::{CameraBehaviorComponent, update_camera_behaviors};
    let (mut world, camera, _) = setup();
    world.get_mut(camera).unwrap().components.insert(
        CameraBehaviorComponent::TYPE_NAME.to_owned(),
        json!({"shake":{"trauma":0.0,"offset":[3.0,4.0]},"future":42}),
    );
    let physics = physics(&mut world);
    update_camera_behaviors(&mut world, 1.0 / 60.0);
    assert!(update_orbit_cameras(&mut world, physics.world(), 1.0 / 60.0).is_empty());
    assert_eq!(
        world.get(camera).unwrap().components[CameraBehaviorComponent::TYPE_NAME]["future"],
        42
    );
    let before = world.world_transform(camera).unwrap();
    assert!(clear_camera_orbit(&mut world, camera));
    update_camera_behaviors(&mut world, 1.0 / 60.0);
    assert_near(
        world.world_transform(camera).unwrap().position,
        before.position,
    );
}

#[test]
fn adding_then_removing_orbit_before_a_step_retains_the_old_shake_accounting() {
    use sindri_scene::{CameraBehaviorComponent, update_camera_behaviors};
    let (mut world, camera, _) = setup();
    world
        .get_mut(camera)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position = [3.0, 4.0, 6.0];
    world.get_mut(camera).unwrap().components.insert(
        CameraBehaviorComponent::TYPE_NAME.to_owned(),
        json!({"shake":{"trauma":0.0,"offset":[3.0,4.0]}}),
    );
    assert!(clear_camera_orbit(&mut world, camera));
    update_camera_behaviors(&mut world, 1.0 / 60.0);
    assert_near(
        world.world_transform(camera).unwrap().position,
        [0.0, 0.0, 6.0],
    );
}

#[test]
fn a_solid_at_the_focus_pulls_in_to_zero_with_a_finite_orientation() {
    let (mut world, camera, _) = setup();
    blocker(&mut world, "enclosing", 0.0, false, 1);
    let physics = physics(&mut world);
    assert!(update_orbit_cameras(&mut world, physics.world(), 1.0 / 60.0).is_empty());
    let pose = world.world_transform(camera).unwrap();
    assert_near(pose.position, [0.0; 3]);
    assert_near(pose.forward(), [0.0, 0.0, -1.0]);
}
