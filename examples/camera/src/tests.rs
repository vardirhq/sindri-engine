use super::*;
use sindri_core::{EntityId, SceneComponent, SceneEntityId};
use sindri_scene::CameraBehaviorComponent;

fn demo(width: u32, height: u32) -> EngineHost<CameraGame> {
    let document = SceneDocument::from_json(SCENE_JSON).unwrap();
    let extractor = demo_extractor().unwrap();
    extractor.validate(&document, sindri_core::UnknownComponentPolicy::Reject).unwrap();
    let mut engine = EngineHost::new(
        CameraGame::new(extractor.components().clone()), FixedStepConfig::default(),
    ).unwrap();
    *engine.world_mut() = World::from_scene(&document).unwrap().world;
    engine.set_viewport(width, height);
    engine.start().unwrap();
    frames(&mut engine, 2);
    engine
}
fn frames(engine: &mut EngineHost<CameraGame>, count: u32) {
    for _ in 0..count {
        engine.advance(Duration::from_secs_f32(1.0 / 60.0)).unwrap();
    }
}
fn entity(engine: &EngineHost<CameraGame>, id: &str) -> EntityId {
    engine.world().entity_for_source_id(&SceneEntityId::new(id).unwrap()).unwrap()
}
fn position(engine: &EngineHost<CameraGame>, id: &str) -> [f32; 3] {
    engine.world().world_transform(entity(engine, id)).unwrap().position
}
fn press(engine: &mut EngineHost<CameraGame>, key: Key) {
    engine.queue_input(InputEvent::KeyPressed(key));
    frames(engine, 1);
    engine.queue_input(InputEvent::KeyReleased(key));
    frames(engine, 1);
}
fn behavior(engine: &EngineHost<CameraGame>) -> &serde_json::Value {
    &engine.world().get(entity(engine, "camera")).unwrap().components[CameraBehaviorComponent::TYPE_NAME]
}

#[test]
fn follow_dead_zone_and_camera_relative_guide_use_the_engine_path() {
    let mut engine = demo(960, 540);
    engine.queue_input(InputEvent::KeyPressed(Key::ArrowRight));
    frames(&mut engine, 10);
    assert!(position(&engine, "camera")[0].abs() < 0.00001);
    frames(&mut engine, 50);
    assert!(position(&engine, "camera")[0] > 0.5);
    let camera = position(&engine, "camera");
    let guide = position(&engine, "dead-zone");
    assert!((camera[0] - guide[0]).abs() < 0.00001);
    assert!((camera[1] - guide[1]).abs() < 0.00001);
}

#[test]
fn independent_modes_restore_settings_and_confinement_is_observable() {
    let mut engine = demo(960, 540);
    press(&mut engine, Key::F);
    assert!(behavior(&engine)["follow"].is_null());
    assert!(!engine.world().is_active(entity(&engine, "dead-zone")));
    engine.queue_input(InputEvent::KeyPressed(Key::ArrowRight));
    frames(&mut engine, 120);
    assert!(position(&engine, "camera")[0].abs() < 0.00001);
    press(&mut engine, Key::F);
    frames(&mut engine, 180);
    assert!((position(&engine, "camera")[0] - 8.0).abs() < 0.0001);
    press(&mut engine, Key::B);
    frames(&mut engine, 180);
    assert!(position(&engine, "camera")[0] > 10.0);
    press(&mut engine, Key::Z);
    assert!(behavior(&engine)["follow"]["dead_zone"][0].as_f64().unwrap().abs() < f64::EPSILON);
    press(&mut engine, Key::S);
    assert!((behavior(&engine)["follow"]["smoothing"].as_f64().unwrap() - 1000.0).abs() < f64::EPSILON);
    engine.queue_input(InputEvent::KeyReleased(Key::ArrowRight));
    press(&mut engine, Key::R);
    frames(&mut engine, 240);
    assert!(position(&engine, "target")[0].abs() < 0.00001);
    assert!(position(&engine, "camera")[0].abs() <= 1.251);
    assert!((behavior(&engine)["follow"]["smoothing"].as_f64().unwrap() - 5.0).abs() < f64::EPSILON);
    assert!(!behavior(&engine)["confine"].is_null());
}

#[test]
fn shake_can_be_disabled_and_impacts_decay_without_drift() {
    let mut engine = demo(960, 540);
    press(&mut engine, Key::Space);
    assert!(behavior(&engine)["shake"]["trauma"].as_f64().unwrap() > 0.5);
    frames(&mut engine, 90);
    assert!(position(&engine, "camera")[0].abs() < 0.00001);
    press(&mut engine, Key::K);
    press(&mut engine, Key::Space);
    assert!(behavior(&engine)["shake"]["trauma"].as_f64().unwrap().abs() < f64::EPSILON);
    assert!(position(&engine, "camera")[0].abs() < 0.00001);
}

#[test]
fn phone_touch_buttons_toggle_modes_and_move_the_target() {
    let mut engine = demo(390, 844);
    // Follow is at x=-0.6*aspect, y=-0.49 in the viewport overlay.
    engine.queue_input(InputEvent::TouchStarted { id: 1, x: 78.0, y: 628.78 });
    frames(&mut engine, 1);
    engine.queue_input(InputEvent::TouchEnded { id: 1 });
    frames(&mut engine, 1);
    assert!(behavior(&engine)["follow"].is_null());
    engine.queue_input(InputEvent::TouchStarted { id: 2, x: 323.7, y: 759.6 });
    frames(&mut engine, 30);
    assert!(position(&engine, "target")[0] > 2.0);
    engine.queue_input(InputEvent::TouchEnded { id: 2 });
    frames(&mut engine, 1);
}

#[test]
fn automatic_tour_crosses_the_bounds_and_manual_input_stops_it() {
    let mut engine = demo(960, 540);
    press(&mut engine, Key::T);
    frames(&mut engine, 180);
    assert!(position(&engine, "target")[0] > 11.5);
    assert!(position(&engine, "target")[1] > 1.0);
    assert!(position(&engine, "camera")[0] <= 8.00001);
    engine.queue_input(InputEvent::KeyPressed(Key::ArrowLeft));
    frames(&mut engine, 1);
    let label = &engine.world().get(entity(&engine, "tour-label")).unwrap().components["sindri.ui.text"]["text"];
    assert_eq!(label.as_str(), Some("START TOUR"));
}
