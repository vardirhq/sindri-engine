use serde_json::json;
use sindri_core::{EntityData, SceneComponent, World};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_scene::{CameraBehaviorComponent, SceneExtractor};

fn registry() -> sindri_core::ComponentSchemaRegistry {
    let mut registry = SceneExtractor::new()
        .expect("the builtin components register")
        .components()
        .clone();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
}

/// The trauma the one behavior camera ends a pass on, after a script ran
/// `body` in its `start`, from `trauma`.
fn trauma_after(trauma: f64, body: &str) -> f64 {
    let mut world = World::default();
    let camera = world.spawn(EntityData {
        components: [
            (
                "sindri.camera".to_owned(),
                json!({
                    "projection": "orthographic",
                    "vertical_size": 8.0,
                    "near": 0.1,
                    "far": 100.0
                }),
            ),
            (
                CameraBehaviorComponent::TYPE_NAME.to_owned(),
                json!({ "shake": { "trauma": trauma } }),
            ),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    world.spawn(EntityData {
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": "camera.decay", "script": "CameraHit" }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert(
        "camera.decay",
        format!("script CameraHit {{ fn start() {{ {body} }} }}"),
    );
    let report = Scripts::new().advance(
        &mut world,
        &registry(),
        ScriptFrame::new(&sources, &InputState::default(), 1.0 / 60.0),
    );
    assert!(report.is_quiet(), "{report:?}");
    world.get(camera).unwrap().components[CameraBehaviorComponent::TYPE_NAME]["shake"]["trauma"]
        .as_f64()
        .unwrap()
}

#[test]
fn an_impact_shakes_at_least_as_hard_as_it_asks_and_never_adds_up() {
    // Raised to the impact, from nothing and from less.
    assert!((trauma_after(0.0, "Camera.impact(0.4);") - 0.4).abs() < 1.0e-6);
    assert!((trauma_after(0.2, "Camera.impact(0.5);") - 0.5).abs() < 1.0e-6);
    // A smaller one during a bigger shake changes nothing, however many.
    let many = "for i in 0..20 { Camera.impact(0.1); }";
    assert!((trauma_after(0.6, many) - 0.6).abs() < 1.0e-6);
    assert!((trauma_after(0.0, many) - 0.1).abs() < 1.0e-6);
    // Never past the hardest shake there is.
    assert!((trauma_after(0.0, "Camera.impact(3.0);") - 1.0).abs() < 1.0e-6);
    // Where adding the same hits would have pinned it there.
    let added = "for i in 0..20 { Camera.add_trauma(0.1); }";
    assert!((trauma_after(0.0, added) - 1.0).abs() < 1.0e-6);
}

#[test]
fn shake_takes_strength_then_frequency_then_decay_as_documented() {
    let mut world = World::default();
    let camera = world.spawn(EntityData {
        components: [
            (
                "sindri.camera".to_owned(),
                json!({ "projection": "orthographic", "vertical_size": 8.0, "near": 0.1, "far": 100.0 }),
            ),
            (CameraBehaviorComponent::TYPE_NAME.to_owned(), json!({})),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    world.spawn(EntityData {
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": "camera.decay", "script": "CameraHit" }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert(
        "camera.decay",
        r"script CameraHit { fn start() { Camera.shake(0.3, 40.0, 1.5); } }",
    );
    let report = Scripts::new().advance(
        &mut world,
        &registry(),
        ScriptFrame::new(&sources, &InputState::default(), 1.0 / 60.0),
    );
    assert!(report.is_quiet(), "{report:?}");
    let shake = &world.get(camera).unwrap().components[CameraBehaviorComponent::TYPE_NAME]["shake"];
    assert_eq!(shake["strength"].as_f64(), Some(0.3_f32.into()));
    assert_eq!(shake["frequency"].as_f64(), Some(40.0));
    assert_eq!(shake["decay"].as_f64(), Some(1.5));
}

#[test]
fn a_script_adds_trauma_to_the_authored_camera_behavior() {
    let mut world = World::default();
    let camera = world.spawn(EntityData {
        components: [
            (
                "sindri.camera".to_owned(),
                json!({
                    "projection": "orthographic",
                    "vertical_size": 8.0,
                    "near": 0.1,
                    "far": 100.0
                }),
            ),
            (
                CameraBehaviorComponent::TYPE_NAME.to_owned(),
                json!({ "shake": { "trauma": 0.2 } }),
            ),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    world.spawn(EntityData {
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": "camera.decay", "script": "CameraHit" }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert(
        "camera.decay",
        r"script CameraHit { fn start() { Camera.add_trauma(0.5); } }",
    );

    let report = Scripts::new().advance(
        &mut world,
        &registry(),
        ScriptFrame::new(&sources, &InputState::default(), 1.0 / 60.0),
    );
    assert!(report.is_quiet(), "{report:?}");
    let trauma = world.get(camera).unwrap().components[CameraBehaviorComponent::TYPE_NAME]["shake"]
        ["trauma"]
        .as_f64()
        .unwrap();
    assert!((trauma - 0.7).abs() < 1.0e-6);
}

#[test]
fn camera_trauma_without_one_behavior_camera_is_a_runtime_error() {
    let mut world = World::default();
    world.spawn(EntityData {
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": "camera.decay", "script": "CameraHit" }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert(
        "camera.decay",
        r"script CameraHit { fn start() { Camera.add_trauma(0.5); } }",
    );

    let report = Scripts::new().advance(
        &mut world,
        &registry(),
        ScriptFrame::new(&sources, &InputState::default(), 1.0 / 60.0),
    );
    assert_eq!(report.failures.len(), 1, "{report:?}");
    assert!(
        format!("{:?}", report.failures).contains("exactly one authored behavior camera"),
        "{report:?}"
    );
}
