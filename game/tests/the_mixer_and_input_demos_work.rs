//! Sound Mixer and Input Actions, played the way a browser plays them: read off
//! disk, styled by their own stylesheets for the viewport, and driven through
//! the session every export runs. Each is checked at a desktop, a phone held
//! upright and a phone on its side, because a demo that only fits one of them
//! is not finished.

use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use sindri_causeway::{Session, extractor};
use sindri_core::{EntityId, LoadedScenes, SceneDocument, SceneEntityId, World};
use sindri_decay::{ScriptSources, Scripts};
use sindri_platform::{InputEvent, InputState, Key, MouseButton};
use sindri_scene::UiTextSizes;

const STEP: f32 = 1.0 / 60.0;
const SCREENS: [[f32; 2]; 3] = [[1280.0, 720.0], [390.0, 844.0], [844.0, 390.0]];

struct Demo {
    session: Session,
    world: World,
    input: InputState,
    size: [f32; 2],
}

impl Demo {
    fn open(project: &str, scene: &str, script: &str, sheet: &str, size: [f32; 2]) -> Self {
        let root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples"))
            .join(project)
            .join("assets");
        let read = |id: &str| std::fs::read_to_string(root.join(id)).expect("the file reads");
        let document = SceneDocument::from_json(&read(scene)).expect("the scene parses");
        let mut world = World::default();
        LoadedScenes::new()
            .enter_keeping_identities(&mut world, scene, &document)
            .expect("the scene loads");
        let mut sources = ScriptSources::new();
        sources.insert(script, read(script));
        let registry = extractor().expect("the schemas register");
        let failures = Scripts::new().compile(&world, registry.components(), &sources);
        assert!(failures.is_empty(), "{failures:?}");
        let sheets: BTreeMap<String, String> = [(sheet.to_owned(), read(sheet))].into();
        let style = weave::compose(sheet, &sheets).expect("the stylesheet composes");
        let session = Session::with_sources(registry.components().clone(), sources)
            .with_scenes(vec![(scene.to_owned(), document)], LoadedScenes::new())
            .with_styles(vec![style]);
        let mut demo = Self {
            session,
            world,
            input: InputState::default(),
            size,
        };
        let view = demo.viewport();
        demo.session
            .settle_styles(&mut demo.world, view)
            .expect("the stylesheet applies");
        demo.play(10);
        demo
    }

    fn mixer(size: [f32; 2]) -> Self {
        Self::open(
            "audio",
            "mixer.scene",
            "mixer.decay",
            "ui/mixer.weave",
            size,
        )
    }

    fn input(size: [f32; 2]) -> Self {
        Self::open(
            "input",
            "input.scene",
            "input.decay",
            "ui/input.weave",
            size,
        )
    }

    fn viewport(&self) -> weave::Viewport {
        weave::Viewport {
            width: self.size[0],
            height: self.size[1],
        }
    }

    fn step(&mut self) {
        self.session
            .step(
                &mut self.world,
                &self.input,
                (self.size[0], self.size[1]),
                STEP,
            )
            .expect("the step runs")
            .log();
        self.input.begin_frame(Duration::from_secs_f32(STEP));
        let view = self.viewport();
        let undo = self
            .session
            .style(&mut self.world, view)
            .expect("the draw styles");
        self.session
            .record_drawn(&self.world, view, UiTextSizes::new())
            .expect("the draw records");
        if let Some(undo) = undo {
            undo.undo(&mut self.world);
        }
    }

    fn play(&mut self, steps: usize) {
        for _ in 0..steps {
            self.step();
        }
    }

    fn id(&self, name: &str) -> EntityId {
        self.world
            .entity_for_source_id(&SceneEntityId::new(name).expect("a valid id"))
            .unwrap_or_else(|| panic!("no {name}"))
    }

    fn text(&self, name: &str) -> String {
        self.world.get(self.id(name)).expect("there").components["sindri.ui.text"]["text"]
            .as_str()
            .expect("words")
            .to_owned()
    }

    fn rect(&self, name: &str) -> sindri_scene::ScreenRect {
        self.session
            .screen_ui()
            .rect(self.id(name))
            .unwrap_or_else(|| panic!("{name} is not on screen"))
    }

    /// Every named element lies wholly on the screen.
    fn fits(&self, names: &[&str]) {
        let half_wide = self.size[0] / self.size[1];
        for name in names {
            let rect = self.rect(name);
            let (x, y) = (rect.center[0].abs(), rect.center[1].abs());
            assert!(
                x + rect.size[0] / 2.0 <= half_wide + 1.0e-3
                    && y + rect.size[1] / 2.0 <= 1.0 + 1.0e-3,
                "{name} runs off a {:?} screen: {rect:?}",
                self.size
            );
        }
    }

    fn click_at(&mut self, overlay: [f32; 2]) {
        let half = self.size[1] / 2.0;
        let (x, y) = (
            self.size[0] / 2.0 + overlay[0] * half,
            half - overlay[1] * half,
        );
        self.input.apply(InputEvent::PointerMoved { x, y });
        self.input
            .apply(InputEvent::ButtonPressed(MouseButton::Left));
        self.step();
        self.input
            .apply(InputEvent::ButtonReleased(MouseButton::Left));
        self.play(2);
    }

    /// A click `along` an element, from its left edge (0) to its right (1).
    fn click(&mut self, name: &str, along: f32) {
        let rect = self.rect(name);
        self.click_at([
            rect.center[0] - rect.size[0] / 2.0 + rect.size[0] * along,
            rect.center[1],
        ]);
    }

    fn key(&mut self, key: Key) {
        self.input.apply(InputEvent::KeyPressed(key));
        self.step();
        self.input.apply(InputEvent::KeyReleased(key));
        self.play(2);
    }

    /// Holds `keys` for `steps` steps; how far the ship went across.
    fn hold(&mut self, keys: &[Key], steps: usize) -> f32 {
        let ship = self.id("ship");
        let start = self.world.world_transform(ship).expect("a ship").position[0];
        for key in keys {
            self.input.apply(InputEvent::KeyPressed(*key));
        }
        self.play(steps);
        for key in keys {
            self.input.apply(InputEvent::KeyReleased(*key));
        }
        self.play(1);
        self.world.world_transform(ship).expect("a ship").position[0] - start
    }
}

fn percent(text: &str) -> f32 {
    text.trim_end_matches('%').parse().expect("a percentage")
}

#[test]
fn the_mixer_fits_every_screen() {
    for size in SCREENS {
        Demo::mixer(size).fits(&[
            "title",
            "panel",
            "master",
            "music",
            "effects",
            "master-value",
            "effects-label",
            "shot",
            "boom",
            "status",
            "hint",
        ]);
    }
}

#[test]
fn the_mixer_moves_buses_by_keyboard_and_pointer_and_plays_through_them() {
    let mut demo = Demo::mixer([390.0, 844.0]);
    // Master has focus when the mixer opens; the arrows move it a step at a time.
    for _ in 0..4 {
        demo.key(Key::ArrowLeft);
    }
    assert_eq!(demo.text("master-value"), "80%");
    demo.key(Key::ArrowDown);
    demo.key(Key::ArrowLeft);
    assert_eq!(demo.text("music-value"), "95%");
    assert_eq!(demo.text("master-value"), "80%", "only the focused bus");

    // A press a quarter along the effects track sets it there.
    demo.click("effects", 0.25);
    let effects = percent(&demo.text("effects-value"));
    assert!((effects - 25.0).abs() <= 5.0, "{effects}");
    demo.click("shot", 0.5);
    assert!(
        demo.text("status")
            .starts_with("A shot, on the effects bus at"),
        "{}",
        demo.text("status")
    );
}

#[test]
fn the_input_demo_fits_every_screen_and_its_arena_fills_the_gap() {
    for size in SCREENS {
        let demo = Demo::input(size);
        demo.fits(&[
            "title",
            "bindings",
            "move-label",
            "boost-label",
            "status",
            "rebind",
            "add",
        ]);
        // The arena sits between the bindings and the status line, in the
        // world, whatever shape the screen is.
        let arena = demo
            .world
            .world_transform(demo.id("arena"))
            .expect("an arena");
        let high = if size[0] >= size[1] {
            12.0
        } else {
            12.0 * size[1] / size[0]
        };
        let unit = high / 2.0;
        let top = (arena.position[1] + arena.scale[1] / 2.0) / unit;
        let bottom = (arena.position[1] - arena.scale[1] / 2.0) / unit;
        let bindings = demo.rect("bindings");
        let status = demo.rect("status");
        assert!(
            top <= bindings.center[1] - bindings.size[1] / 2.0 + 1.0e-3,
            "{size:?}"
        );
        assert!(
            bottom >= status.center[1] + status.size[1] / 2.0 - 1.0e-3,
            "{size:?}"
        );
        assert!(arena.scale[1] > 1.0, "{size:?}: a usable arena");
    }
}

#[test]
fn the_ship_moves_by_action_and_boost_is_rebound_while_it_runs() {
    let mut demo = Demo::input([1280.0, 720.0]);
    assert_eq!(demo.text("boost-label"), "BOOST   Space   pad south");
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
    demo.key(Key::K);
    assert!(demo.text("status").starts_with("Press a key"));
    demo.key(Key::J);
    assert_eq!(demo.text("status"), "BOOST is now on key.J.");
    assert_eq!(demo.text("boost-label"), "BOOST   J   pad south");
    let rebound = demo.hold(&[Key::A, Key::J], 20);
    assert!(rebound < -boosted * 0.9, "J boosts now: {rebound}");
    let plain = demo.hold(&[Key::A, Key::Space], 20);
    assert!(plain > rebound * 0.6, "and Space no longer does: {plain}");
}
