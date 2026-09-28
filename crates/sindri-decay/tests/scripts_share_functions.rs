//! A function declared outside any script, in one file, called by name from
//! scripts in others.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFailure, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;

const VIEW: &str = "fn half(size: f32) -> f32 { return size * 0.5; }
fn quarter(size: f32) -> f32 { return half(half(size)); }";

const SHOWS: &str = "script Shows {
    let view_size: f32 = 12.0;
    fn update(dt: f32) { this.transform.position.x = quarter(this.view_size); }
}";

const PLAIN: &str = "script Plain { fn update(dt: f32) { this.transform.position.x = 7.0; } }";

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
    sources.insert("view.decay", VIEW);
    sources.insert("shows.decay", SHOWS);
    sources.insert("plain.decay", PLAIN);
    sources
}

#[test]
fn a_script_calls_a_function_another_file_declares() {
    let mut world = World::default();
    let shows = scripted(&mut world, "shows.decay", "Shows");
    let mut scripts = Scripts::new();
    let failures = frame(&mut scripts, &mut world, &sources());
    assert!(failures.is_empty(), "{failures:?}");
    assert!((x(&world, shows) - 3.0).abs() < 1e-6);
}

#[test]
fn changing_the_function_changes_what_its_callers_run() {
    let mut world = World::default();
    let shows = scripted(&mut world, "shows.decay", "Shows");
    let mut scripts = Scripts::new();
    let mut sources = sources();
    assert!(frame(&mut scripts, &mut world, &sources).is_empty());
    // The caller's own text is unchanged; only the file it calls into is.
    sources.insert(
        "view.decay",
        "fn half(size: f32) -> f32 { return size; }
         fn quarter(size: f32) -> f32 { return half(half(size)); }",
    );
    let failures = frame(&mut scripts, &mut world, &sources);
    assert!(failures.is_empty(), "{failures:?}");
    assert!(
        (x(&world, shows) - 12.0).abs() < 1e-6,
        "{}",
        x(&world, shows)
    );
}

#[test]
fn a_broken_helper_file_fails_its_callers_and_no_one_else() {
    let mut world = World::default();
    scripted(&mut world, "shows.decay", "Shows");
    let plain = scripted(&mut world, "plain.decay", "Plain");
    let mut sources = sources();
    sources.insert(
        "view.decay",
        "fn half(size: f32) -> f32 { return size * nope; }
         fn quarter(size: f32) -> f32 { return half(half(size)); }",
    );
    let mut scripts = Scripts::new();
    let failures = frame(&mut scripts, &mut world, &sources);
    assert!(
        failures.iter().any(|failure| matches!(
            failure,
            ScriptFailure::Compile { asset, diagnostics }
                if asset == "view.decay" && diagnostics.iter().any(|d| d.contains("nope"))
        )),
        "{failures:?}"
    );
    assert_eq!(failures.len(), 1, "only the caller fails: {failures:?}");
    assert!((x(&world, plain) - 7.0).abs() < 1e-6);
}
