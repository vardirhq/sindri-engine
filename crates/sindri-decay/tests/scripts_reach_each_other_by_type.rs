//! One script naming another by type: reading and writing its live fields, and
//! sending it messages — across files, against a real world.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFailure, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;

const BOLT: &str = "script Bolt {
    var damage: f32 = 1.0;
    let armour: f32 = 2.0;
    var hits: f32 = 0.0;
    fn update(dt: f32) {
        this.transform.position.x = damage;
        this.transform.position.y = hits;
    }
    fn hit(amount: f32) {
        this.hits += amount;
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

/// A bolt, and a turret whose update is `turret`, in two files.
fn world(turret: &str) -> (World, EntityId, EntityId, ScriptSources) {
    let mut world = World::default();
    let bolt = scripted(&mut world, "Bolt", "bolt.decay", "Bolt");
    let tower = scripted(&mut world, "Turret", "turret.decay", "Turret");
    let mut sources = ScriptSources::new();
    sources.insert("bolt.decay", BOLT);
    sources.insert(
        "turret.decay",
        format!("script Turret {{ fn update(dt: f32) {{ {turret} }} }}"),
    );
    (world, bolt, tower, sources)
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

fn position(world: &World, entity: EntityId) -> [f32; 3] {
    world
        .get(entity)
        .and_then(|data| data.transform_3d)
        .expect("a transform")
        .position
}

#[test]
fn one_script_writes_another_s_live_field() {
    let (mut world, bolt, tower, sources) = world(
        "let bolt = Bolt.on(World.find(\"Bolt\"));
         if bolt != null { bolt.damage = bolt.damage + 4.0; this.transform.position.x = bolt.damage; }",
    );
    let failures = frames(&mut world, &sources, 2);
    assert!(failures.is_empty(), "{failures:?}");
    // Two frames of +4 on a live field that started at 1. The bolt runs before
    // the turret, so it shows the turret's write a frame later: 5, while the
    // turret has already made it 9.
    assert!((position(&world, tower)[0] - 9.0).abs() < 1e-6);
    assert!(
        (position(&world, bolt)[0] - 5.0).abs() < 1e-6,
        "{:?}",
        position(&world, bolt)
    );
}

#[test]
fn a_message_is_delivered_after_the_pass_in_order() {
    let (mut world, bolt, _, sources) = world(
        "let bolt = Bolt.on(World.find(\"Bolt\"));
         bolt.hit(1.0);
         bolt.hit(2.0);",
    );
    let failures = frames(&mut world, &sources, 2);
    assert!(failures.is_empty(), "{failures:?}");
    // Frame one delivers 3 after the bolt's update; frame two's update shows
    // it, and frame two delivers 3 more that the next update would show.
    assert!(
        (position(&world, bolt)[1] - 3.0).abs() < 1e-6,
        "{:?}",
        position(&world, bolt)
    );
}

#[test]
fn looking_for_a_script_an_entity_does_not_run_finds_nothing() {
    let (mut world, _, tower, sources) =
        world("if Bolt.on(this.entity) == null { this.transform.position.x = 1.0; }");
    let failures = frames(&mut world, &sources, 1);
    assert!(failures.is_empty(), "{failures:?}");
    assert!((position(&world, tower)[0] - 1.0).abs() < 1e-6);
}

#[test]
fn a_let_cannot_be_changed_once_its_script_has_started() {
    let (mut world, _, _, sources) =
        world("let bolt = Bolt.on(World.find(\"Bolt\")); bolt.armour = 9.0;");
    let failures = frames(&mut world, &sources, 2);
    assert!(
        failures
            .iter()
            .any(|failure| failure.to_string().contains("`armour` is a `let`")),
        "{failures:?}"
    );
}

#[test]
fn a_message_that_sends_itself_forever_is_stopped_and_reported() {
    let mut world = World::default();
    scripted(&mut world, "Echo", "echo.decay", "Echo");
    scripted(&mut world, "Other", "echo.decay", "Echo");
    let mut sources = ScriptSources::new();
    sources.insert(
        "echo.decay",
        "script Echo {
            fn update(dt: f32) { ping(); }
            fn ping() {
                let other = Echo.on(World.find(\"Other\"));
                let echo = Echo.on(World.find(\"Echo\"));
                if other != null && other != this.entity { other.ping(); }
                if echo != null && echo != this.entity { echo.ping(); }
            }
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
fn a_mistake_about_another_script_does_not_compile() {
    let (mut world, _, _, sources) = world("Bolt.on(this.entity).damge = 1.0;");
    let failures = frames(&mut world, &sources, 1);
    assert!(
        failures
            .iter()
            .any(|failure| failure.to_string().contains("damge")),
        "{failures:?}"
    );
}
