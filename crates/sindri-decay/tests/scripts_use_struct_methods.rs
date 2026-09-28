//! A struct's methods, declared in one file, asked of values in another.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFailure, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;

/// A struct with methods, one of which calls another.
const CARDS: &str = "struct Card {
    weight: f32,
    fn doubled() -> Card { return Card(weight: this.weight * 2.0); }
    fn score() -> f32 { return this.doubled().weight + 1.0; }
}";

const USES: &str = "script Uses {
    fn update(dt: f32) { this.transform.position.x = Card(weight: 3.0).score(); }
}";

fn registry() -> ComponentSchemaRegistry {
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
}

fn scripted(world: &mut World, source: &str, script: &str) -> EntityId {
    world.spawn(EntityData {
        name: Some(script.to_owned()),
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

fn frame(scripts: &mut Scripts, world: &mut World, sources: &ScriptSources) -> Vec<ScriptFailure> {
    scripts
        .advance(
            world,
            &registry(),
            ScriptFrame::new(sources, &InputState::default(), 1.0 / 60.0),
        )
        .failures
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
    sources.insert("cards.decay", CARDS);
    sources.insert("uses.decay", USES);
    sources
}

#[test]
fn a_script_asks_a_value_a_method_another_file_declares() {
    let mut world = World::default();
    let uses = scripted(&mut world, "uses.decay", "Uses");
    let mut scripts = Scripts::new();
    let failures = frame(&mut scripts, &mut world, &sources());
    assert!(failures.is_empty(), "{failures:?}");
    assert!((x(&world, uses) - 7.0).abs() < 1e-6, "{}", x(&world, uses));
}

#[test]
fn changing_a_method_changes_what_its_callers_run() {
    let mut world = World::default();
    let uses = scripted(&mut world, "uses.decay", "Uses");
    let mut scripts = Scripts::new();
    let mut sources = sources();
    assert!(frame(&mut scripts, &mut world, &sources).is_empty());
    sources.insert(
        "cards.decay",
        "struct Card {
            weight: f32,
            fn doubled() -> Card { return Card(weight: this.weight * 3.0); }
            fn score() -> f32 { return this.doubled().weight + 1.0; }
        }",
    );
    let failures = frame(&mut scripts, &mut world, &sources);
    assert!(failures.is_empty(), "{failures:?}");
    assert!((x(&world, uses) - 10.0).abs() < 1e-6, "{}", x(&world, uses));
}

#[test]
fn a_method_the_struct_does_not_have_is_refused_where_it_is_called() {
    let mut world = World::default();
    scripted(&mut world, "uses.decay", "Uses");
    let mut sources = sources();
    sources.insert(
        "uses.decay",
        "script Uses { fn update(dt: f32) { let s = Card(weight: 1.0).halved(); } }",
    );
    let mut scripts = Scripts::new();
    let failures = frame(&mut scripts, &mut world, &sources);
    assert!(
        failures.iter().any(|failure| matches!(
            failure,
            ScriptFailure::Compile { diagnostics, .. }
                if diagnostics.iter().any(|d| d.contains("no method `halved`"))
        )),
        "{failures:?}"
    );
}
