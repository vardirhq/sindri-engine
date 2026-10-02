//! A host that asks for timings gets one per script, over every entity that
//! runs it; a host that does not ask gets none.
use serde_json::json;
use sindri_core::{ComponentSchemaRegistry, EntityData, SceneComponent, World};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;

fn run(measuring: bool) -> Vec<sindri_decay::ScriptTiming> {
    let mut registry = ComponentSchemaRegistry::default();
    registry.register::<ScriptComponent>("Script").unwrap();
    let mut world = World::default();
    for script in ["Spinner", "Spinner", "Idle"] {
        world.spawn(EntityData {
            components: [(
                ScriptComponent::TYPE_NAME.to_owned(),
                json!({"source": "timed.decay", "script": script}),
            )]
            .into_iter()
            .collect(),
            ..EntityData::default()
        });
    }
    let mut sources = ScriptSources::new();
    sources.insert(
        "timed.decay",
        "script Spinner { var turns: f32 = 0.0; fn update(dt: f32) { turns = turns + dt; } }\n\
         script Idle { fn start() {} }",
    );
    let input = InputState::default();
    let mut scripts = Scripts::new();
    scripts.set_measuring(measuring);
    let report = scripts.advance(
        &mut world,
        &registry,
        ScriptFrame::new(&sources, &input, 1.0 / 60.0),
    );
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    report.timings
}

#[test]
fn each_script_is_timed_once_over_every_entity_running_it() {
    let timings = run(true);
    let runs: Vec<(&str, u32)> = timings
        .iter()
        .map(|timing| (timing.script.as_str(), timing.runs))
        .collect();
    assert_eq!(runs, [("Spinner", 2), ("Idle", 1)]);
    assert!(timings.iter().all(|timing| timing.source == "timed.decay"));
}

#[test]
fn nothing_is_timed_unless_asked() {
    assert!(run(false).is_empty());
}
