//! Explicit camera controls compile and enforce their runtime contracts.

use serde_json::json;
use sindri_core::{EntityData, SceneComponent, SceneEntityId, Transform3D, World};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_scene::{CameraComponent, CameraOrbitComponent, SceneExtractor, set_camera_orbit};

fn run(body: &str) -> (bool, EntityData, EntityData) {
    let mut world = World::default();
    let target = world.spawn(EntityData {
        source_id: Some(SceneEntityId::new("target").unwrap()),
        name: Some("Target".to_owned()),
        transform_3d: Some(Transform3D::default()),
        ..EntityData::default()
    });
    let camera=world.spawn(EntityData { transform_3d:Some(Transform3D::default()),
        components:[(CameraComponent::TYPE_NAME.to_owned(),json!({"projection":"perspective","vertical_fov_degrees":60.0,"near":0.1,"far":100.0})),
            (ScriptComponent::TYPE_NAME.to_owned(),json!({"source":"camera.decay","script":"Rig"}))].into_iter().collect(),
        ..EntityData::default() });
    assert!(set_camera_orbit(&mut world, camera, target, 0.0, 0.0, 6.0));
    let before = world.get(camera).unwrap().clone();
    let mut registry = SceneExtractor::new().unwrap().components().clone();
    registry.register::<ScriptComponent>("Script").unwrap();
    let mut sources = ScriptSources::new();
    sources.insert(
        "camera.decay",
        format!("script Rig {{ fn start() {{ {body} }} }}"),
    );
    let report = Scripts::new().advance(
        &mut world,
        &registry,
        ScriptFrame::new(&sources, &InputState::default(), 1.0 / 60.0),
    );
    assert!(
        report
            .failures
            .iter()
            .all(|failure| matches!(failure, sindri_decay::ScriptFailure::Runtime { .. })),
        "{report:?}"
    );
    (
        report.failures.is_empty(),
        before,
        world.get(camera).unwrap().clone(),
    )
}

#[test]
fn invalid_camera_calls_preserve_the_authored_settings_and_pose() {
    for body in [
        "Camera.perspective_fov(this.entity, 0.0);",
        "Camera.perspective_fov(this.entity, 180.0);",
        "Camera.orbit(this.entity, this.entity, 0.0, 0.0, 6.0);",
        "Camera.orbit(this.entity, World.find(\"Target\"), 0.0, 2.0, 6.0);",
        "Camera.orbit(this.entity, World.find(\"Target\"), 0.0, 0.0, 0.0);",
        "Camera.orbit_collision(this.entity, 1.5, 0.2);",
        "Camera.orbit_collision(this.entity, -1.0, 0.2);",
        "Camera.orbit_collision(this.entity, 1.0, 6.0);",
        "Camera.orbit_smoothing(this.entity, -1.0);",
    ] {
        let (ok, before, after) = run(body);
        assert!(!ok, "{body}");
        assert_eq!(before, after, "{body}");
    }
}

#[test]
fn explicit_camera_controls_do_not_require_a_2d_behavior_camera() {
    let (ok, _, after) = run(
        "Camera.perspective_fov(this.entity, 70.0); Camera.orbit_offset(this.entity, Vec3(1.0, 2.0, 3.0)); Camera.orbit_smoothing(this.entity, 0.0); Camera.orbit_collision(this.entity, 0.0, 0.1);",
    );
    assert!(ok);
    assert!(
        (after.components[CameraComponent::TYPE_NAME]["vertical_fov_degrees"]
            .as_f64()
            .unwrap()
            - 70.0)
            .abs()
            < 1.0e-6
    );
    let orbit = &after.components[CameraOrbitComponent::TYPE_NAME];
    assert_eq!(orbit["offset"], json!([1.0, 2.0, 3.0]));
    assert!((orbit["smoothing"].as_f64().unwrap()).abs() < f64::EPSILON);
    let (ok, before, after) = run("Camera.clear_orbit(this.entity);");
    assert!(ok);
    assert!(
        !after
            .components
            .contains_key(CameraOrbitComponent::TYPE_NAME)
    );
    assert_eq!(before.transform_3d, after.transform_3d);
}
