//! The Weave Control Room, played the way a browser plays it.
//!
//! The project is read off disk, styled by its own stylesheet for the viewport,
//! and driven through the same session every export runs: clicks land where
//! the styled elements are drawn, keys and typed characters arrive as a host
//! delivers them. What is checked is what a person would see change.

use std::{collections::BTreeMap, path::Path, time::Duration};

use sindri_causeway::{Session, extractor};
use sindri_core::{EntityId, LoadedScenes, SceneDocument, SceneEntityId, World};
use sindri_decay::{ScriptSources, Scripts};
use sindri_platform::{InputEvent, InputState, Key, MouseButton};
use sindri_scene::UiTextSizes;

const STEP: f32 = 1.0 / 60.0;

fn project() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../examples/ui/assets"
    ))
}

fn read(id: &str) -> String {
    std::fs::read_to_string(project().join(id)).expect("the project file reads")
}

struct Demo {
    session: Session,
    world: World,
    input: InputState,
    size: [f32; 2],
}

impl Demo {
    fn open(width: f32, height: f32) -> Self {
        let document = SceneDocument::from_json(&read("widgets.scene")).expect("the scene parses");
        let mut world = World::default();
        LoadedScenes::new()
            .enter_keeping_identities(&mut world, "widgets.scene", &document)
            .expect("the scene loads");
        let mut sources = ScriptSources::new();
        sources.insert("ui.decay", read("ui.decay"));
        let scene = extractor().expect("the schemas register");
        let failures = Scripts::new().compile(&world, scene.components(), &sources);
        assert!(failures.is_empty(), "{failures:?}");

        let weave: BTreeMap<String, String> =
            [("ui/widgets.weave".to_owned(), read("ui/widgets.weave"))].into();
        let sheet = weave::compose("ui/widgets.weave", &weave).expect("the stylesheet composes");
        let session = Session::with_sources(scene.components().clone(), sources)
            .with_scenes(
                vec![("widgets.scene".to_owned(), document)],
                LoadedScenes::new(),
            )
            .with_styles(vec![sheet]);
        let mut demo = Self {
            session,
            world,
            input: InputState::default(),
            size: [width, height],
        };
        let view = demo.viewport();
        demo.session
            .settle_styles(&mut demo.world, view)
            .expect("the stylesheet applies");
        demo.play(10);
        demo
    }

    fn viewport(&self) -> weave::Viewport {
        weave::Viewport {
            width: self.size[0],
            height: self.size[1],
        }
    }

    /// One fixed step and one draw, as the browser host runs them.
    fn step(&mut self) {
        self.session
            .step(
                &mut self.world,
                &self.input,
                (self.size[0], self.size[1]),
                STEP,
            )
            .expect("the step runs");
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

    fn field(&self, name: &str, component: &str, field: &str) -> serde_json::Value {
        self.world.get(self.id(name)).expect("there").components[component][field].clone()
    }

    fn offset(&self) -> f32 {
        #[allow(clippy::cast_possible_truncation)]
        let offset = self
            .field("archive", "sindri.ui.scroll", "offset")
            .as_f64()
            .expect("a number") as f32;
        offset
    }

    /// Where `name` was drawn, in overlay units: centre and size.
    fn rect(&self, name: &str) -> sindri_scene::ScreenRect {
        self.session
            .screen_ui()
            .rect(self.id(name))
            .unwrap_or_else(|| panic!("{name} is not on screen"))
    }

    fn pixel(&self, overlay: [f32; 2]) -> [f32; 2] {
        let half = self.size[1] / 2.0;
        [
            self.size[0] / 2.0 + overlay[0] * half,
            half - overlay[1] * half,
        ]
    }

    fn click_at(&mut self, pixel: [f32; 2]) {
        self.input.apply(InputEvent::PointerMoved {
            x: pixel[0],
            y: pixel[1],
        });
        self.input
            .apply(InputEvent::ButtonPressed(MouseButton::Left));
        self.step();
        self.input
            .apply(InputEvent::ButtonReleased(MouseButton::Left));
        self.play(2);
    }

    fn click(&mut self, name: &str) {
        let pixel = self.pixel(self.rect(name).center);
        self.click_at(pixel);
    }

    fn key(&mut self, key: Key) {
        self.input.apply(InputEvent::KeyPressed(key));
        self.step();
        self.input.apply(InputEvent::KeyReleased(key));
        self.play(2);
    }

    fn type_text(&mut self, text: &str) {
        for c in text.chars() {
            self.input.apply(InputEvent::TextInput(c));
        }
        self.play(2);
    }

    fn wheel(&mut self, name: &str, pixels: f32) {
        let [x, y] = self.pixel(self.rect(name).center);
        self.input.apply(InputEvent::PointerMoved { x, y });
        self.input
            .apply(InputEvent::Scrolled { x: 0.0, y: -pixels });
        self.play(2);
    }

    fn top(&self, name: &str) -> f32 {
        let rect = self.rect(name);
        rect.center[1] + rect.size[1] / 2.0
    }

    fn bottom(&self, name: &str) -> f32 {
        let rect = self.rect(name);
        rect.center[1] - rect.size[1] / 2.0
    }
}

#[test]
fn the_switches_flip_the_locked_one_does_not_and_their_words_follow() {
    let mut demo = Demo::open(960.0, 540.0);
    assert_eq!(demo.text("toggle-text"), "Flight assist  / on");
    demo.click("toggle");
    assert_eq!(demo.field("toggle", "sindri.ui.toggle", "checked"), false);
    assert_eq!(demo.text("toggle-text"), "Flight assist  / off");

    demo.click("checkbox");
    assert_eq!(demo.field("checkbox", "sindri.ui.toggle", "checked"), true);
    assert_eq!(demo.text("checkbox-text"), "Mission telemetry  / logging");

    demo.click("autopilot");
    assert_eq!(
        demo.field("autopilot", "sindri.ui.toggle", "checked"),
        false
    );
    assert_eq!(
        demo.text("focus-readout"),
        "FOCUS / none",
        "a locked switch takes no focus"
    );
}

#[test]
fn a_callsign_is_typed_in_any_script_and_submitted_once() {
    let mut demo = Demo::open(960.0, 540.0);
    demo.click("callsign");
    assert_eq!(demo.text("focus-readout"), "FOCUS / callsign / typing");
    demo.type_text("Nøva 猫 7");
    assert_eq!(
        demo.text("callsign-text"),
        "Nøva 猫 7_",
        "a caret while it has the keyboard"
    );
    // Space is a letter here, not a press of the field.
    demo.key(Key::Space);
    demo.type_text(" ");
    demo.key(Key::Backspace);
    demo.key(Key::Enter);
    assert_eq!(demo.text("status"), "CALLSIGN / Nøva 猫 7 confirmed");
    demo.key(Key::Escape);
    assert_eq!(
        demo.text("callsign-text"),
        "Nøva 猫 7",
        "no caret once it lets go"
    );
    assert_eq!(demo.text("focus-readout"), "FOCUS / none");
}

#[test]
fn tab_walks_the_controls_in_the_order_they_are_written() {
    let mut demo = Demo::open(960.0, 540.0);
    let mut seen = Vec::new();
    for _ in 0..4 {
        demo.key(Key::Tab);
        seen.push(demo.text("focus-readout"));
    }
    assert_eq!(
        seen,
        [
            "FOCUS / flight assist",
            "FOCUS / mission telemetry",
            "FOCUS / callsign / typing",
            "FOCUS / archive row 1",
        ]
    );
    demo.key(Key::Space);
    assert_eq!(demo.text("status"), "SELECTED / Guidance Kernel");
}

/// The archive, at every shape it is drawn: its first row starts at its top,
/// the wheel stops where its last row reaches its bottom, a row scrolled out
/// of view cannot be clicked, and a locked row cannot be picked.
#[test]
fn the_archive_scrolls_exactly_as_far_as_its_rows_go_at_every_size() {
    for size in [
        [960.0, 540.0],
        [390.0, 844.0],
        [740.0, 360.0],
        [820.0, 1180.0],
    ] {
        let mut demo = Demo::open(size[0], size[1]);
        // The region's padding and border, and no more: ten pixels.
        let pad = 10.0 * 2.0 / size[1];
        let gap = demo.top("archive") - demo.top("row-0");
        assert!(
            (0.0..pad).contains(&gap),
            "{size:?}: the first row starts at the top, {gap} below it"
        );
        let first = demo.rect("row-0");
        demo.wheel("archive", 10_000.0);
        let offset = demo.offset();
        assert!(offset > 0.5, "{size:?}: scrolled {offset}");
        let tail = demo.bottom("row-18") - demo.bottom("archive");
        assert!(
            (0.0..pad).contains(&tail),
            "{size:?}: the last row ends at the bottom, {tail} above it"
        );
        // Where the first row was is now a later row; where it is now is
        // above the region, clipped, and clicks there reach nothing.
        demo.click_at(demo.pixel(demo.rect("row-0").center));
        assert_eq!(demo.text("status"), "READY / Select a control", "{size:?}");
        demo.click_at(demo.pixel(first.center));
        assert_ne!(demo.text("status"), "READY / Select a control", "{size:?}");

        demo.wheel("archive", -10_000.0);
        assert!(demo.offset().abs() < 1.0e-5, "{size:?}: back to the top");
        demo.wheel("archive", 180.0);
        let before = demo.text("status");
        demo.click("row-9");
        assert_eq!(
            demo.text("status"),
            before,
            "{size:?}: a locked row is not picked"
        );
    }
}

#[test]
fn a_finger_drags_the_archive_without_picking_the_row_it_started_on() {
    let mut demo = Demo::open(390.0, 844.0);
    let [x, y] = demo.pixel(demo.rect("row-2").center);
    demo.input.apply(InputEvent::TouchStarted { id: 1, x, y });
    demo.step();
    for travel in 1..=12 {
        #[allow(clippy::cast_precision_loss)]
        let moved = y - travel as f32 * 12.0;
        demo.input
            .apply(InputEvent::TouchMoved { id: 1, x, y: moved });
        demo.step();
    }
    demo.input.apply(InputEvent::TouchEnded { id: 1 });
    demo.play(2);
    assert!(demo.offset() > 0.2, "dragged to {}", demo.offset());
    assert_eq!(demo.text("status"), "READY / Select a control");

    // A drag the window loses is let go of, not finished as a click.
    let [x, y] = demo.pixel(demo.rect("row-6").center);
    demo.input.apply(InputEvent::TouchStarted { id: 2, x, y });
    demo.step();
    demo.input.apply(InputEvent::FocusChanged(false));
    demo.play(2);
    assert_eq!(demo.text("status"), "READY / Select a control");
}
