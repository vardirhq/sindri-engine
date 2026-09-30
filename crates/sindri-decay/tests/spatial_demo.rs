//! The feature project executes the real Decay query APIs and UI controls.
use sindri_core::{EntityId, SceneDocument, SceneEntityId, World};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::{InputEvent, InputState, Key};
use sindri_scene::{SceneExtractor, ScreenExtent, ScreenUi};
use std::time::Duration;

struct Demo {
    world: World,
    extractor: SceneExtractor,
    sources: ScriptSources,
    scripts: Scripts,
    input: InputState,
    screen: ScreenUi,
    extent: ScreenExtent,
}
impl Demo {
    fn new(width: f32, height: f32) -> Self {
        let document = SceneDocument::from_json(include_str!(
            "../../../examples/spatial/assets/spatial.scene"
        ))
        .unwrap();
        let mut extractor = SceneExtractor::new().unwrap();
        extractor.register::<ScriptComponent>("Script").unwrap();
        extractor
            .validate(&document, sindri_core::UnknownComponentPolicy::Reject)
            .unwrap();
        let world = World::from_scene(&document).unwrap().world;
        let mut sources = ScriptSources::new();
        sources.insert(
            "spatial.decay",
            include_str!("../../../examples/spatial/assets/spatial.decay"),
        );
        let mut scripts = Scripts::new();
        assert!(
            scripts
                .compile(&world, extractor.components(), &sources)
                .is_empty()
        );
        let mut demo = Self {
            world,
            extractor,
            sources,
            scripts,
            input: InputState::default(),
            screen: ScreenUi::new(),
            extent: ScreenExtent::new(width, height),
        };
        demo.step();
        demo.step();
        demo
    }
    fn id(&self, name: &str) -> EntityId {
        self.world
            .entity_for_source_id(&SceneEntityId::new(name).unwrap())
            .unwrap()
    }
    fn text(&self, name: &str) -> &str {
        self.world.get(self.id(name)).unwrap().components["sindri.ui.text"]["text"]
            .as_str()
            .unwrap()
    }
    fn step(&mut self) {
        self.screen
            .update(
                &mut self.world,
                self.extractor.components(),
                self.extent,
                self.input.presses(),
            )
            .unwrap();
        let report = self.scripts.advance(
            &mut self.world,
            self.extractor.components(),
            ScriptFrame::new(&self.sources, &self.input, 1.0 / 60.0).with_screen_ui(&self.screen),
        );
        assert!(report.failures.is_empty(), "{:?}", report.failures);
        self.input.begin_frame(Duration::from_millis(16));
    }
    fn press(&mut self, key: Key) {
        self.input.apply(InputEvent::KeyPressed(key));
        self.step();
        self.input.apply(InputEvent::KeyReleased(key));
        self.step();
    }
}

#[test]
fn project_shows_nearest_stable_ties_boundary_and_world_space_order() {
    let demo = Demo::new(960.0, 540.0);
    assert_eq!(demo.text("nearest-label"), "nearest: A");
    let order: Vec<_> = demo.text("results-label").split_whitespace().collect();
    assert_eq!(order, ["1:A", "2:B", "3:D", "4:E"]);
    assert!(demo.text("query-label").contains("4 inside"));
    let child = demo.world.world_transform(demo.id("D")).unwrap().position;
    assert!((child[0] - 1.0).abs() < 0.00001);
    assert!((child[1] - 1.5).abs() < 0.00001);
}

#[test]
fn zero_radius_stays_empty_while_nearest_searches_globally() {
    let mut demo = Demo::new(960.0, 540.0);
    for _ in 0..4 {
        demo.press(Key::Q);
    }
    assert_eq!(demo.text("results-label"), "within_radius: []");
    assert_eq!(demo.text("nearest-label"), "nearest: A");
    assert!(demo.text("query-label").contains("radius 0.0"));
}

#[test]
fn inactive_target_and_disabled_parent_disappear_from_the_query() {
    let mut demo = Demo::new(960.0, 540.0);
    demo.press(Key::A);
    assert_eq!(demo.text("nearest-label"), "nearest: B");
    demo.press(Key::P);
    let order: Vec<_> = demo.text("results-label").split_whitespace().collect();
    assert_eq!(order, ["1:B", "2:E"]);
    assert!(demo.world.is_active(demo.id("A-ghost")));
    assert!(demo.world.is_active(demo.id("D-ghost")));
    demo.press(Key::R);
    assert_eq!(demo.text("nearest-label"), "nearest: A");
}

#[test]
fn rotating_the_parent_changes_nearest_using_composed_transforms() {
    let mut demo = Demo::new(960.0, 540.0);
    demo.press(Key::O);
    assert_eq!(demo.text("nearest-label"), "nearest: D");
    assert!(demo.text("results-label").starts_with("1:D"));
    let child = demo.world.world_transform(demo.id("D")).unwrap().position;
    assert!((child[0] - (2.0 - 2.0_f32.sqrt())).abs() < 0.00001);
    assert!((child[1] - 0.5).abs() < 0.00001);
}

#[test]
fn tag_selection_exposes_ally_and_null_results() {
    let mut demo = Demo::new(960.0, 540.0);
    demo.press(Key::T);
    assert_eq!(demo.text("nearest-label"), "nearest: F");
    assert_eq!(demo.text("results-label"), "within_radius: []");
    demo.press(Key::E);
    assert_eq!(demo.text("results-label").trim(), "1:F");
    demo.press(Key::T);
    assert_eq!(demo.text("nearest-label"), "nearest: null");
    assert_eq!(demo.text("results-label"), "within_radius: []");
}

#[test]
fn phone_touch_controls_change_the_tag_and_move_the_query_origin() {
    let mut demo = Demo::new(390.0, 844.0);
    demo.input.apply(InputEvent::TouchStarted {
        id: 1,
        x: 195.0,
        y: 649.88,
    });
    demo.step();
    demo.input.apply(InputEvent::TouchEnded { id: 1 });
    demo.step();
    assert_eq!(demo.text("nearest-label"), "nearest: F");
    demo.input.apply(InputEvent::TouchStarted {
        id: 2,
        x: 331.5,
        y: 776.48,
    });
    for _ in 0..30 {
        demo.step();
    }
    let probe = demo
        .world
        .world_transform(demo.id("probe"))
        .unwrap()
        .position;
    assert!(probe[0] > 1.0);
}
