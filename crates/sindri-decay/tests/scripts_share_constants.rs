//! A constant declared in one file, used by name in scripts in others.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFailure, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;

/// Shared constants, one worked out from another file's, and one the file
/// keeps to itself.
const TUNING: &str = "shared const ARENA: f32 = 12.0;
shared const EDGE: f32 = ARENA - MARGIN;
const PRIVATE: f32 = 99.0;";

const LAYOUT: &str = "shared const MARGIN: f32 = 2.0;";

const USES: &str = "const OWN: f32 = 0.5;
script Uses {
    fn update(dt: f32) { this.transform.position.x = EDGE + OWN; }
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
    sources.insert("tuning.decay", TUNING);
    sources.insert("layout.decay", LAYOUT);
    sources.insert("uses.decay", USES);
    sources
}

#[test]
fn a_script_uses_a_constant_worked_out_across_files() {
    let mut world = World::default();
    let uses = scripted(&mut world, "uses.decay", "Uses");
    let mut scripts = Scripts::new();
    let failures = frame(&mut scripts, &mut world, &sources());
    assert!(failures.is_empty(), "{failures:?}");
    assert!((x(&world, uses) - 10.5).abs() < 1e-6, "{}", x(&world, uses));
}

#[test]
fn changing_a_constant_recompiles_what_uses_it() {
    let mut world = World::default();
    let uses = scripted(&mut world, "uses.decay", "Uses");
    let mut scripts = Scripts::new();
    let mut sources = sources();
    assert!(frame(&mut scripts, &mut world, &sources).is_empty());
    sources.insert("layout.decay", "shared const MARGIN: f32 = 4.0;");
    let failures = frame(&mut scripts, &mut world, &sources);
    assert!(failures.is_empty(), "{failures:?}");
    assert!((x(&world, uses) - 8.5).abs() < 1e-6, "{}", x(&world, uses));
}

#[test]
fn a_files_own_constant_stays_its_own() {
    let mut world = World::default();
    scripted(&mut world, "uses.decay", "Uses");
    let mut sources = sources();
    sources.insert(
        "uses.decay",
        "script Uses { fn update(dt: f32) { this.transform.position.x = PRIVATE; } }",
    );
    let mut scripts = Scripts::new();
    let failures = frame(&mut scripts, &mut world, &sources);
    assert!(
        failures.iter().any(|failure| matches!(
            failure,
            ScriptFailure::Compile { diagnostics, .. }
                if diagnostics.iter().any(|d| d.contains("unknown name `PRIVATE`"))
        )),
        "{failures:?}"
    );
}

#[test]
fn a_shared_constant_declared_twice_is_refused_where_it_is_used() {
    let mut world = World::default();
    scripted(&mut world, "uses.decay", "Uses");
    let mut sources = sources();
    sources.insert("twice.decay", "shared const EDGE: f32 = 1.0;");
    let mut scripts = Scripts::new();
    let failures = frame(&mut scripts, &mut world, &sources);
    assert!(
        failures.iter().any(|failure| matches!(
            failure,
            ScriptFailure::Compile { diagnostics, .. }
                if diagnostics.iter().any(|d| d.contains("more than one file"))
        )),
        "{failures:?}"
    );
}
