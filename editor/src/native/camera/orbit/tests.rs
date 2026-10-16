use sindri_core::{EntityData, SceneEntityId, Transform3D};

use super::*;

fn spawn(world: &mut World, name: &str, position: [f32; 3]) -> EntityId {
    world.spawn(EntityData {
        source_id: Some(SceneEntityId::new(name).unwrap()),
        transform_3d: Some(Transform3D {
            position,
            ..Transform3D::default()
        }),
        ..EntityData::default()
    })
}

fn fixture() -> (World, EntityId, EntityId) {
    let mut world = World::default();
    let target = spawn(&mut world, "target", [1.0, 2.0, 3.0]);
    let camera = spawn(&mut world, "camera", [0.0, 3.0, 6.0]);
    let data = world.get_mut(camera).unwrap();
    data.components.insert(
        CameraComponent::TYPE_NAME.into(),
        crate::native::scene_io::scene_extractor()
            .components()
            .default_payload(CameraComponent::TYPE_NAME)
            .unwrap()
            .clone(),
    );
    set_orbit(
        &mut world,
        camera,
        CameraOrbitComponent::new(SceneEntityId::new("target").unwrap()),
    );
    (world, camera, target)
}

fn set_orbit(world: &mut World, camera: EntityId, orbit: CameraOrbitComponent) {
    world.get_mut(camera).unwrap().components.insert(
        CameraOrbitComponent::TYPE_NAME.into(),
        serde_json::to_value(orbit).unwrap(),
    );
}

#[test]
fn requested_orbit_and_yaw_circle_follow_world_axes_on_parented_entities() {
    let (mut world, camera, target) = fixture();
    let parent = spawn(&mut world, "parent", [10.0, 0.0, -2.0]);
    let transform = world
        .get_mut(parent)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap();
    transform.rotation = glam::Quat::from_rotation_y(TAU / 4.0).to_array();
    transform.scale = [2.0; 3];
    world.set_parent(target, Some(parent)).unwrap();
    world.set_parent(camera, Some(parent)).unwrap();
    let mut orbit = CameraOrbitComponent::new(SceneEntityId::new("target").unwrap());
    orbit.offset = [0.0, 1.0, 0.0];
    orbit.yaw = TAU / 4.0;
    orbit.pitch = 0.4;
    orbit.distance = 5.0;
    set_orbit(&mut world, camera, orbit);
    let guide = Guide::from_world(&world, camera).unwrap();
    assert!(guide.focus.distance(Vec3::new(16.0, 5.0, -4.0)) < 1.0e-5);
    let expected = guide.focus + Vec3::new(5.0 * 0.4_f32.cos(), 5.0 * 0.4_f32.sin(), 0.0);
    assert!(guide.desired.distance(expected) < 1.0e-5);
    assert!(
        guide.current.distance(Vec3::from_array(
            world.world_transform(camera).unwrap().position
        )) < 1.0e-5
    );
    let segments: Vec<_> = guide.yaw_segments().collect();
    assert_eq!(segments.len(), 48);
    assert!(segments[0][0].distance(segments[47][1]) < 1.0e-5);
    for point in segments.into_iter().flatten() {
        assert!((point.distance(guide.focus) - 5.0).abs() < 1.0e-5);
        assert!((point.y - guide.desired.y).abs() < 1.0e-5);
    }
}

#[test]
fn guides_track_current_runtime_poses_without_changing_requested_distance() {
    let (mut world, camera, target) = fixture();
    let before = Guide::from_world(&world, camera).unwrap();
    world
        .get_mut(camera)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position = [1.0, 2.5, 4.0];
    world
        .get_mut(target)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position = [4.0, 2.0, 3.0];
    let after = Guide::from_world(&world, camera).unwrap();
    assert!(after.current.distance(Vec3::new(1.0, 2.5, 4.0)) < 1.0e-6);
    assert!(after.focus.distance(before.focus + Vec3::X * 3.0) < 1.0e-6);
    assert!(after.desired.distance(before.desired + Vec3::X * 3.0) < 1.0e-6);
    assert!((after.desired.distance(after.focus) - 6.0).abs() < 1.0e-6);
}

#[test]
fn invalid_orbits_and_unusable_targets_have_no_guides() {
    let (mut world, camera, target) = fixture();
    world.get_mut(target).unwrap().disabled = true;
    assert!(Guide::from_world(&world, camera).is_none());
    world.get_mut(target).unwrap().disabled = false;
    world.set_parent(target, Some(camera)).unwrap();
    assert!(Guide::from_world(&world, camera).is_none());
    world.set_parent(target, None).unwrap();
    for distance in [0.0, -1.0] {
        let mut orbit = CameraOrbitComponent::new(SceneEntityId::new("target").unwrap());
        orbit.distance = distance;
        set_orbit(&mut world, camera, orbit);
        assert!(Guide::from_world(&world, camera).is_none());
    }
    set_orbit(
        &mut world,
        camera,
        CameraOrbitComponent::new(SceneEntityId::new("missing").unwrap()),
    );
    assert!(Guide::from_world(&world, camera).is_none());
    set_orbit(
        &mut world,
        camera,
        CameraOrbitComponent::new(SceneEntityId::new("camera").unwrap()),
    );
    assert!(Guide::from_world(&world, camera).is_none());
    world
        .get_mut(camera)
        .unwrap()
        .components
        .remove(CameraComponent::TYPE_NAME);
    assert!(Guide::from_world(&world, camera).is_none());
}

#[test]
fn overflowed_focus_is_hidden_instead_of_projecting_nonfinite_geometry() {
    let (mut world, camera, target) = fixture();
    world
        .get_mut(target)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position[0] = f32::MAX;
    let mut orbit = CameraOrbitComponent::new(SceneEntityId::new("target").unwrap());
    orbit.offset[0] = f32::MAX;
    set_orbit(&mut world, camera, orbit);
    assert!(Guide::from_world(&world, camera).is_none());
}

#[test]
fn the_focus_remains_visible_when_the_current_camera_is_behind_the_scene_eye() {
    let (mut world, camera, target) = fixture();
    world
        .get_mut(target)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position = [0.0; 3];
    world
        .get_mut(camera)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position = [0.0, 0.0, 14.0];
    let guide = Guide::from_world(&world, camera).unwrap();
    let view_projection = sindri_render::perspective_projection(0.8, 1.0, 0.1, 100.0)
        * sindri_render::look_at(Vec3::new(0.0, 0.0, 12.0), Vec3::ZERO, Vec3::Y);
    let rect = Rect::from_min_size(eframe::egui::Pos2::ZERO, Vec2::splat(500.0));
    assert!(project_point(rect, view_projection, guide.current).is_none());
    assert!(project_point(rect, view_projection, guide.focus).is_some());
    assert!(project_point(rect, view_projection, guide.desired).is_some());
    let clipped = project_segment(rect, view_projection, guide.focus, guide.current).unwrap();
    assert!(
        clipped
            .into_iter()
            .all(|point| point.x.is_finite() && point.y.is_finite())
    );
}
