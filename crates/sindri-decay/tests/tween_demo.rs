//! The Pages example is an authored project with the real gameplay scripts.
use std::time::Duration;
use sindri_core::{SceneDocument, SceneEntityId, World};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::{InputEvent, InputState, Key};
use sindri_scene::{SceneExtractor, ScreenExtent, ScreenUi};

#[test]
fn demo_project_compiles_animates_and_obeys_playback_controls() {
    let document = SceneDocument::from_json(include_str!("../../../examples/tween/assets/tween.scene")).expect("scene");
    let mut world = World::from_scene(&document).expect("world").world;
    let mut extractor = SceneExtractor::new().expect("schemas");
    extractor.register::<ScriptComponent>("Script").expect("script schema");
    extractor.validate(&document, sindri_core::UnknownComponentPolicy::Reject).expect("valid project");
    let mut sources = ScriptSources::new();
    sources.insert("scripts/tween-demo.decay", include_str!("../../../examples/tween/assets/scripts/tween-demo.decay"));
    let mut scripts = Scripts::new();
    assert!(scripts.compile(&world, extractor.components(), &sources).is_empty());
    let mut screen = ScreenUi::new();
    let mut input = InputState::default();
    let sample = world.entity_for_source_id(&SceneEntityId::new("sample-linear").expect("id")).expect("sample");
    let mut run = |world: &mut World, input: &InputState| {
        screen.update(world, extractor.components(), ScreenExtent::new(960.0, 540.0), input.presses()).expect("screen");
        let report = scripts.advance(world, extractor.components(),
            ScriptFrame::new(&sources, input, 1.0 / 60.0).with_screen_ui(&screen));
        assert!(report.failures.is_empty(), "{:?}", report.failures);
    };
    run(&mut world, &input);
    let start = world.world_transform(sample).expect("sample").position[0];
    for _ in 0..30 { run(&mut world, &input); }
    assert!(world.world_transform(sample).expect("sample").position[0] > start);
    input.apply(InputEvent::KeyPressed(Key::P));
    run(&mut world, &input);
    input.begin_frame(Duration::from_millis(16));
    input.apply(InputEvent::KeyReleased(Key::P));
    run(&mut world, &input);
    let paused = world.world_transform(sample).expect("sample").position[0];
    for _ in 0..30 { run(&mut world, &input); }
    assert!((world.world_transform(sample).expect("sample").position[0] - paused).abs() < 0.00001);
    input.apply(InputEvent::KeyPressed(Key::Space));
    run(&mut world, &input);
    input.begin_frame(Duration::from_millis(16));
    input.apply(InputEvent::KeyReleased(Key::Space));
    for _ in 0..200 { run(&mut world, &input); }
    assert!((world.world_transform(sample).expect("sample").position[0] - 2.6).abs() < 0.00001);
    input.apply(InputEvent::KeyPressed(Key::R));
    run(&mut world, &input);
    input.begin_frame(Duration::from_millis(16));
    input.apply(InputEvent::KeyReleased(Key::R));
    run(&mut world, &input);
    assert!(world.world_transform(sample).expect("sample").position[0] < -2.5);
}
