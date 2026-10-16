//! `Input`: every way the person reaches the game, found under one name.
//!
//! The ways in grew one namespace at a time — `Input` for keys, then
//! `Pointer`, `Touch`, `Gesture`, `Stick`, `Action` and `Gamepad` beside it —
//! and someone looking for touch controls started at `Input` and found only
//! keys. `Input.Stick` and the rest are the same namespaces reached from where
//! people look first, so these tests are about the two spellings agreeing.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptReport, ScriptSources, Scripts};
use sindri_platform::{InputEvent, InputState, Key};

fn registry() -> ComponentSchemaRegistry {
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
}

fn world(script: &str) -> (World, EntityId, ScriptSources) {
    let mut world = World::default();
    let entity = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": "reader.decay", "script": "Reader" }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert("reader.decay", script);
    (world, entity, sources)
}

/// Runs one frame of `script` and gives back what it wrote to its position.
fn run(script: &str, input: &InputState) -> ([f32; 3], ScriptReport) {
    let (mut world, entity, sources) = world(script);
    let report = Scripts::new().advance(
        &mut world,
        &registry(),
        ScriptFrame::new(&sources, input, 0.5),
    );
    let position = world
        .get(entity)
        .and_then(|data| data.transform_3d)
        .expect("the entity kept its transform")
        .position;
    (position, report)
}

fn close(got: [f32; 3], want: [f32; 3]) -> bool {
    got.iter()
        .zip(want)
        .all(|(got, want)| (got - want).abs() < 1.0e-5)
}

#[test]
fn input_keyboard_is_the_keyboard() {
    let script = r#"
        script Reader {
            fn update(dt: f32) {
                this.transform.position.x = Input.Keyboard.axis("ArrowLeft", "ArrowRight");
                if Input.Keyboard.is_down("Space") { this.transform.position.y = 1.0; }
                if Input.Keyboard.just_pressed("Space") { this.transform.position.z = 1.0; }
            }
        }
    "#;
    let mut input = InputState::default();
    input.apply(InputEvent::KeyPressed(Key::ArrowRight));
    input.apply(InputEvent::KeyPressed(Key::Space));

    let (at, report) = run(script, &input);
    assert!(report.is_quiet(), "{report:?}");
    assert!(close(at, [1.0, 1.0, 1.0]), "{at:?}");
}

/// The calls directly on `Input` still read the keyboard, so no script that
/// was written before `Input` gathered anything stops working.
#[test]
fn the_calls_on_input_itself_are_still_the_keyboard() {
    let script = r#"
        script Reader {
            fn update(dt: f32) {
                this.transform.position.x = Input.axis("ArrowLeft", "ArrowRight");
                this.transform.position.y = Input.Keyboard.axis("ArrowLeft", "ArrowRight");
            }
        }
    "#;
    let mut input = InputState::default();
    input.apply(InputEvent::KeyPressed(Key::ArrowLeft));

    let (at, report) = run(script, &input);
    assert!(report.is_quiet(), "{report:?}");
    assert!(close(at, [-1.0, -1.0, 0.0]), "{at:?}");
}

#[test]
fn a_finger_reads_the_same_through_input_and_on_its_own() {
    let script = r"
        script Reader {
            fn update(dt: f32) {
                this.transform.position.x = Input.Pointer.x - Pointer.x + Input.Pointer.x;
                this.transform.position.y = Input.Touch.y(0.0) - Touch.y(0.0) + Input.Touch.y(0.0);
                this.transform.position.z = Input.Touch.count;
                if Input.Stick.held != Stick.held { this.transform.position.z = -1.0; }
                if Input.Stick.x != Stick.x { this.transform.position.z = -1.0; }
                if Input.Gesture.tapped != Gesture.tapped { this.transform.position.z = -1.0; }
            }
        }
    ";
    let mut input = InputState::default();
    input.apply(InputEvent::TouchStarted {
        id: 1,
        x: 30.0,
        y: 20.0,
    });

    let (at, report) = run(script, &input);
    assert!(report.is_quiet(), "{report:?}");
    assert!(close(at, [30.0, 20.0, 1.0]), "{at:?}");
}

#[test]
fn input_gamepad_is_the_pads() {
    let script = r#"
        script Reader {
            fn update(dt: f32) {
                this.transform.position.x = Input.Gamepad.count() + 1.0;
                if Input.Gamepad.is_down(0.0, "south") { this.transform.position.y = 1.0; }
            }
        }
    "#;

    let (at, report) = run(script, &InputState::default());
    assert!(report.is_quiet(), "{report:?}");
    assert!(close(at, [1.0, 0.0, 0.0]), "{at:?}");
}

/// A name `Input` does not gather is a compile error naming `Input`, not a
/// silent zero: there is no `Input.Mouse`, because `Input.Pointer` is the
/// mouse and a finger alike.
#[test]
fn a_way_in_that_does_not_exist_is_refused() {
    let (_, report) = run(
        r"script Reader { fn update(dt: f32) { this.transform.position.x = Input.Mouse.x; } }",
        &InputState::default(),
    );
    let failures = format!("{:?}", report.failures);
    assert!(failures.contains("Mouse"), "{report:?}");
}
