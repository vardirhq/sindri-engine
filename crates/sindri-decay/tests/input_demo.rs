//! The Input Actions example: a ship moved by what the player means, and a
//! boost rebound to another key while it runs.
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
}

impl Demo {
    fn new() -> Self {
        let document =
            SceneDocument::from_json(include_str!("../../../examples/input/assets/input.scene"))
                .unwrap();
        let mut extractor = SceneExtractor::new().unwrap();
        extractor.register::<ScriptComponent>("Script").unwrap();
        extractor
            .validate(&document, sindri_core::UnknownComponentPolicy::Reject)
            .unwrap();
        let world = World::from_scene(&document).unwrap().world;
        let mut sources = ScriptSources::new();
        sources.insert(
            "input.decay",
            include_str!("../../../examples/input/assets/input.decay"),
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
        };
        demo.step();
        demo
    }
    fn id(&self, name: &str) -> EntityId {
        self.world
            .entity_for_source_id(&SceneEntityId::new(name).unwrap())
            .unwrap()
    }
    fn text(&self, name: &str) -> String {
        self.world.get(self.id(name)).unwrap().components["sindri.ui.text"]["text"]
            .as_str()
            .unwrap()
            .to_owned()
    }
    fn x(&self) -> f32 {
        self.world
            .world_transform(self.id("ship"))
            .unwrap()
            .position[0]
    }
    fn step(&mut self) {
        self.screen
            .update(
                &mut self.world,
                self.extractor.components(),
                ScreenExtent::new(960.0, 540.0),
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
    /// Holds `key` for `steps` steps and lets it go; how far the ship went.
    fn hold(&mut self, keys: &[Key], steps: usize) -> f32 {
        let start = self.x();
        for key in keys {
            self.input.apply(InputEvent::KeyPressed(*key));
        }
        for _ in 0..steps {
            self.step();
        }
        for key in keys {
            self.input.apply(InputEvent::KeyReleased(*key));
        }
        self.step();
        self.x() - start
    }
}

#[test]
fn the_ship_moves_by_action_and_boost_is_rebound_while_it_runs() {
    let mut demo = Demo::new();
    assert_eq!(demo.text("boost-label"), "boost: key.Space, gamepad.south");
    let walk = demo.hold(&[Key::D], 20);
    assert!(walk > 0.5, "D moves right: {walk}");
    let back = demo.hold(&[Key::ArrowLeft], 20);
    assert!(back < -0.5, "the arrows are bound too: {back}");
    let boosted = demo.hold(&[Key::D, Key::Space], 20);
    assert!(
        boosted > walk * 2.0,
        "Space boosts: {boosted} against {walk}"
    );

    // K asks for a key; J becomes boost's first binding.
    demo.hold(&[Key::K], 1);
    assert!(demo.text("status").starts_with("Press a key"));
    demo.hold(&[Key::J], 1);
    assert_eq!(demo.text("status"), "BOOST is now on key.J.");
    assert_eq!(demo.text("boost-label"), "boost: key.J, gamepad.south");
    let rebound = demo.hold(&[Key::A, Key::J], 20);
    assert!(rebound < -boosted * 0.9, "J boosts now: {rebound}");
    let plain = demo.hold(&[Key::A, Key::Space], 20);
    assert!(plain > rebound * 0.6, "and Space no longer does: {plain}");
}
