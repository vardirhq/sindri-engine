//! The Sound Mixer example: sliders that move buses, by keyboard and by touch,
//! and effects that play through the bus they name.
use sindri_core::{SceneDocument, SceneEntityId, World};
use sindri_decay::{AudioCommand, ScriptComponent, ScriptFrame, ScriptSources, Scripts};
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
    heard: Vec<AudioCommand>,
}

impl Demo {
    fn new() -> Self {
        let document =
            SceneDocument::from_json(include_str!("../../../examples/audio/assets/mixer.scene"))
                .unwrap();
        let mut extractor = SceneExtractor::new().unwrap();
        extractor.register::<ScriptComponent>("Script").unwrap();
        extractor
            .validate(&document, sindri_core::UnknownComponentPolicy::Reject)
            .unwrap();
        let world = World::from_scene(&document).unwrap().world;
        let mut sources = ScriptSources::new();
        sources.insert(
            "mixer.decay",
            include_str!("../../../examples/audio/assets/mixer.decay"),
        );
        let mut demo = Self {
            world,
            extractor,
            sources,
            scripts: Scripts::new(),
            input: InputState::default(),
            screen: ScreenUi::new(),
            heard: Vec::new(),
        };
        demo.step();
        demo
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
        self.screen
            .read_controls(&mut self.world, &sindri_decay::ui_input(&self.input, 540.0));
        let report = self.scripts.advance(
            &mut self.world,
            self.extractor.components(),
            ScriptFrame::new(&self.sources, &self.input, 1.0 / 60.0).with_screen_ui(&self.screen),
        );
        assert!(report.failures.is_empty(), "{:?}", report.failures);
        self.heard.extend(self.scripts.take_audio_commands());
        self.input.begin_frame(Duration::from_millis(16));
    }
    fn press(&mut self, key: Key) {
        self.input.apply(InputEvent::KeyPressed(key));
        self.step();
        self.input.apply(InputEvent::KeyReleased(key));
        self.step();
    }
    fn tap(&mut self, x: f32, y: f32) {
        // Overlay units to pixels: one unit is half the height, from the middle.
        let (px, py) = (480.0 + x * 270.0, 270.0 - y * 270.0);
        self.input.apply(InputEvent::TouchStarted {
            id: 1,
            x: px,
            y: py,
        });
        self.step();
        self.input.apply(InputEvent::TouchEnded { id: 1 });
        self.step();
    }
    fn text(&self, name: &str) -> String {
        let id = self
            .world
            .entity_for_source_id(&SceneEntityId::new(name).unwrap())
            .unwrap();
        self.world.get(id).unwrap().components["sindri.ui.text"]["text"]
            .as_str()
            .unwrap()
            .to_owned()
    }
}

#[test]
fn sliders_move_their_buses_and_effects_play_through_theirs() {
    let mut demo = Demo::new();
    // Master has focus when the mixer opens; the arrows move it a step at a time.
    for _ in 0..4 {
        demo.press(Key::ArrowLeft);
    }
    assert!((demo.scripts.audio_volume("master") - 0.8).abs() < 1.0e-5);
    assert_eq!(demo.text("master-value"), "80%");
    assert!(demo.heard.contains(&AudioCommand::SetVolume {
        bus: "master".to_owned(),
        volume: 0.8,
    }));

    demo.press(Key::ArrowDown);
    demo.press(Key::ArrowLeft);
    assert!((demo.scripts.audio_volume("music") - 0.95).abs() < 1.0e-5);
    assert!(
        (demo.scripts.audio_volume("master") - 0.8).abs() < 1.0e-5,
        "only the focused bus"
    );

    // A finger on the effects track sets it where it lands: a quarter along.
    demo.tap(0.1 - 0.9 * 0.25, 0.04);
    assert!(
        (demo.scripts.audio_volume("effects") - 0.25).abs() < 0.03,
        "{}",
        demo.scripts.audio_volume("effects")
    );
    demo.heard.clear();
    demo.tap(-0.3, -0.25);
    assert!(
        demo.heard.iter().any(|command| matches!(command,
            AudioCommand::Play { clip, bus, .. } if clip == "audio/shot.wav" && bus == "effects")),
        "{:?}",
        demo.heard
    );
    assert!(
        demo.text("status")
            .starts_with("A shot, on the effects bus at 25%")
    );
}
