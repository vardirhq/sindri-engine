//! The feature project executes the real Decay query APIs and UI controls.
use sindri_core::{EntityId, SceneDocument, SceneEntityId, World};
use sindri_decay::{Physics2d, ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::{InputEvent, InputState, Key};
use sindri_scene::{SceneExtractor, ScenePhysics2d, ScreenExtent, ScreenUi};
use std::time::Duration;

struct Demo {
    world: World,
    extractor: SceneExtractor,
    sources: ScriptSources,
    scripts: Scripts,
    input: InputState,
    screen: ScreenUi,
    extent: ScreenExtent,
    physics: ScenePhysics2d,
}
impl Demo {
    fn new(width: f32, height: f32) -> Self {
        let document = SceneDocument::from_json(include_str!(
            "../../../examples/physics/assets/physics.scene"
        ))
        .unwrap();
        let mut extractor = SceneExtractor::new().unwrap();
        extractor.register::<ScriptComponent>("Script").unwrap();
        extractor
            .validate(&document, sindri_core::UnknownComponentPolicy::Reject)
            .unwrap();
        // Match exported hosting: the opening scene lives under a loaded root.
        let mut world = World::default();
        sindri_core::LoadedScenes::new()
            .enter_keeping_identities(&mut world, "physics.scene", &document)
            .unwrap();
        let mut sources = ScriptSources::new();
        sources.insert(
            "physics.decay",
            include_str!("../../../examples/physics/assets/physics.decay"),
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
            physics: ScenePhysics2d::top_down().unwrap(),
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
        self.physics
            .step(
                &mut self.world,
                self.extractor.components(),
                Duration::from_secs_f32(1.0 / 60.0),
            )
            .unwrap();
        let (physics, events) = self.physics.for_scripts();
        let report = self.scripts.advance(
            &mut self.world,
            self.extractor.components(),
            ScriptFrame::new(&self.sources, &self.input, 1.0 / 60.0)
                .with_screen_ui(&self.screen)
                .with_physics(Physics2d {
                    world: physics,
                    events,
                }),
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
fn the_project_shows_filtered_hits_inside_hits_and_misses() {
    let mut demo = Demo::new(960.0, 540.0);
    assert!(demo.text("result-label").starts_with("Solid / d 3.40"));
    demo.press(Key::S);
    assert!(demo.text("result-label").starts_with("Sensor / d 1.65"));
    demo.press(Key::I);
    assert_eq!(demo.text("result-label"), "Solid / d 0.00 / n 0.0,0.0");
    demo.press(Key::T);
    demo.press(Key::T);
    demo.press(Key::T);
    assert_eq!(demo.text("result-label"), "miss");
    assert!(!demo.world.is_active(demo.id("hit-dot")));
    demo.press(Key::R);
    assert!(demo.text("result-label").starts_with("Solid / d 3.40"));
    for _ in 0..4 {
        demo.press(Key::Q);
    }
    assert_eq!(demo.text("result-label"), "miss");
}

#[test]
fn falling_bodies_bounce_pass_through_sensors_and_can_be_dropped_again() {
    let mut demo = Demo::new(960.0, 540.0);
    let ball = demo.id("ball");
    let mut bounced = false;
    for _ in 0..180 {
        demo.step();
        let y = demo.world.world_transform(ball).unwrap().position[1];
        let vy = demo.physics.world().linear_velocity(ball).unwrap()[1];
        bounced |= y < -1.0 && vy > 1.0;
    }
    assert!(bounced);
    assert!(demo.scripts.blackboard().get("sensor_entries", 0.0) >= 1.0);
    assert!(demo.scripts.blackboard().get("contacts", 0.0) >= 1.0);
    demo.press(Key::B);
    assert!(demo.world.world_transform(ball).unwrap().position[1] > 1.9);
}

#[test]
fn phone_touch_buttons_toggle_sensors_and_turn_the_visible_ray() {
    let mut demo = Demo::new(390.0, 844.0);
    demo.input.apply(InputEvent::TouchStarted {
        id: 1,
        x: 78.0,
        y: 649.88,
    });
    demo.step();
    demo.input.apply(InputEvent::TouchEnded { id: 1 });
    demo.step();
    assert_eq!(demo.text("sensors-label"), "SENSORS ON");
    assert!(demo.text("result-label").starts_with("Sensor / d 1.65"));
    demo.input.apply(InputEvent::TouchStarted {
        id: 2,
        x: 312.0,
        y: 776.48,
    });
    demo.step();
    demo.input.apply(InputEvent::TouchEnded { id: 2 });
    demo.step();
    assert!(demo.text("query-label").contains("angle 0.26"));
}

#[test]
fn a_swept_circle_stops_short_of_the_ray_and_an_area_lists_what_it_holds() {
    let mut demo = Demo::new(960.0, 540.0);
    demo.press(Key::C);
    assert_eq!(demo.text("query-mode-label"), "QUERY CIRCLE CAST");
    // The ray meets the solid circle's face at 3.40; a circle of radius 0.35
    // swept the same way touches it that much sooner.
    assert!(
        demo.text("result-label").starts_with("Solid / d 3.05"),
        "{}",
        demo.text("result-label")
    );
    assert!(demo.world.is_active(demo.id("probe-ring")));

    demo.press(Key::C);
    assert_eq!(demo.text("query-mode-label"), "QUERY AREA");
    for _ in 0..3 {
        demo.press(Key::Q);
    }
    assert_eq!(demo.text("result-label"), "area: Solid");
    for _ in 0..2 {
        demo.press(Key::Q);
    }
    assert!(!demo.text("result-label").contains("Sensor"));
    demo.press(Key::S);
    assert!(
        demo.text("result-label").contains("Sensor"),
        "{}",
        demo.text("result-label")
    );

    demo.press(Key::C);
    assert_eq!(demo.text("query-mode-label"), "QUERY RAY");
    assert!(!demo.world.is_active(demo.id("probe-ring")));
}
