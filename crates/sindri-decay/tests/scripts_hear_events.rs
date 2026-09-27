//! An event declared in one file, emitted by one script, and handled by every
//! script listening — across files, against a real world.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFailure, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;

const EVENTS: &str = "event Scored(points: f32);";

/// Emits one `Scored` a frame, and counts what it hears of its own.
const BALL: &str = "script Ball {
    var heard: f32 = 0.0;
    fn update(dt: f32) {
        Scored.emit(2.0);
        this.transform.position.x = heard;
    }
    on Scored(points) { heard += 1.0; }
}";

/// Adds up the points, and shows the total.
const BOARD: &str = "script Board {
    var total: f32 = 0.0;
    fn update(dt: f32) { this.transform.position.x = total; }
    on Scored(points: f32) {
        total += points;
        this.transform.position.x = total;
    }
}";

fn registry() -> ComponentSchemaRegistry {
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
}

fn scripted(world: &mut World, name: &str, source: &str, script: &str) -> EntityId {
    world.spawn(EntityData {
        name: Some(name.to_owned()),
        transform_3d: Some(Transform3D::default()),
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": source, "script": script }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    })
}

fn frames(world: &mut World, sources: &ScriptSources, count: usize) -> Vec<ScriptFailure> {
    let mut scripts = Scripts::new();
    let mut failures = Vec::new();
    for _ in 0..count {
        failures.extend(
            scripts
                .advance(
                    world,
                    &registry(),
                    ScriptFrame::new(sources, &InputState::default(), 1.0 / 60.0),
                )
                .failures,
        );
    }
    failures
}

fn x(world: &World, entity: EntityId) -> f32 {
    world
        .get(entity)
        .and_then(|data| data.transform_3d)
        .expect("a transform")
        .position[0]
}

fn sources() -> ScriptSources {
    let mut sources = ScriptSources::new();
    sources.insert("events.decay", EVENTS);
    sources.insert("ball.decay", BALL);
    sources.insert("board.decay", BOARD);
    sources
}

#[test]
fn every_listener_hears_an_event_after_the_pass_including_the_sender() {
    let mut world = World::default();
    let ball = scripted(&mut world, "Ball", "ball.decay", "Ball");
    let first = scripted(&mut world, "Board", "board.decay", "Board");
    let second = scripted(&mut world, "Other board", "board.decay", "Board");
    let failures = frames(&mut world, &sources(), 3);
    assert!(failures.is_empty(), "{failures:?}");
    // Three frames, two points each, heard by both boards.
    assert!(
        (x(&world, first) - 6.0).abs() < 1e-6,
        "{}",
        x(&world, first)
    );
    assert!((x(&world, second) - 6.0).abs() < 1e-6);
    // The ball hears its own too, after its update: it shows the two before.
    assert!((x(&world, ball) - 2.0).abs() < 1e-6, "{}", x(&world, ball));
}

#[test]
fn an_event_nobody_handles_goes_nowhere_quietly() {
    let mut world = World::default();
    scripted(&mut world, "Ball", "ball.decay", "Ball");
    let mut sources = sources();
    sources.insert(
        "ball.decay",
        "script Ball { fn update(dt: f32) { Scored.emit(1.0); } }",
    );
    let failures = frames(&mut world, &sources, 2);
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn an_event_that_emits_itself_forever_is_stopped_and_reported() {
    let mut world = World::default();
    scripted(&mut world, "Echo", "echo.decay", "Echo");
    let mut sources = ScriptSources::new();
    sources.insert(
        "echo.decay",
        "event Ping();
         script Echo {
             fn update(dt: f32) { Ping.emit(); }
             on Ping() { Ping.emit(); }
         }",
    );
    let failures = frames(&mut world, &sources, 1);
    assert!(
        failures
            .iter()
            .any(|failure| matches!(failure, ScriptFailure::MessagesDidNotSettle { .. })),
        "{failures:?}"
    );
}

#[test]
fn an_event_declared_in_two_files_does_not_compile() {
    let mut world = World::default();
    scripted(&mut world, "Ball", "ball.decay", "Ball");
    let mut sources = sources();
    sources.insert("more.decay", "event Scored(points: f32);");
    let failures = frames(&mut world, &sources, 1);
    assert!(
        failures
            .iter()
            .any(|failure| failure.to_string().contains("more than one file")),
        "{failures:?}"
    );
}

#[test]
fn a_mistake_about_an_event_in_another_file_does_not_compile() {
    let mut world = World::default();
    scripted(&mut world, "Ball", "ball.decay", "Ball");
    let mut sources = sources();
    sources.insert(
        "ball.decay",
        "script Ball { fn update(dt: f32) { Scored.emit(\"two\"); } }",
    );
    let failures = frames(&mut world, &sources, 1);
    assert!(
        failures.iter().any(|failure| failure
            .to_string()
            .contains("cannot assign `String` to `f32`")),
        "{failures:?}"
    );
}
