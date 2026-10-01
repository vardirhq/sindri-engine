//! Collision layers asked for by the names the scene's physics world gives
//! them, and a name it does not give heard about rather than masking nothing.
use serde_json::json;
use sindri_core::{ComponentSchemaRegistry, EntityData, SceneComponent, World};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;

fn run(source: &str) -> (Scripts, Vec<String>) {
    let mut registry = ComponentSchemaRegistry::default();
    registry.register::<ScriptComponent>("Script").unwrap();
    let mut world = World::default();
    world.spawn(EntityData {
        components: [(
            "sindri.physics2d.world".to_owned(),
            json!({"gravity": [0.0, -9.81], "layers": ["ground", "hero", "pickups"]}),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    world.spawn(EntityData {
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({"source":"layers.decay", "script":"Probe"}),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert("layers.decay", source);
    let input = InputState::default();
    let mut scripts = Scripts::new();
    let report = scripts.advance(
        &mut world,
        &registry,
        ScriptFrame::new(&sources, &input, 1.0 / 60.0),
    );
    let failures = report.failures.iter().map(ToString::to_string).collect();
    (scripts, failures)
}

#[test]
fn a_name_is_its_bit_and_several_are_their_union() {
    let (scripts, failures) = run(r#"
        script Probe {
            fn start() {
                Game.set("ground", Physics.layer("ground"));
                Game.set("pickups", Physics.layer("pickups"));
                Game.set("both", Physics.mask(["ground", "pickups"]));
                Game.set("none", Physics.mask([]));
            }
        }
    "#);
    assert!(failures.is_empty(), "{failures:?}");
    for (key, expected) in [
        ("ground", 1.0),
        ("pickups", 4.0),
        ("both", 5.0),
        ("none", 0.0),
    ] {
        assert!(
            (scripts.blackboard().get(key, -1.0) - expected).abs() < 1.0e-9,
            "{key}"
        );
    }
}

#[test]
fn a_name_the_world_does_not_give_is_an_error_that_lists_the_ones_it_does() {
    let (_, failures) = run(r#"
        script Probe {
            fn start() {
                Game.set("typo", Physics.layer("gruond"));
            }
        }
    "#);
    assert!(
        failures
            .iter()
            .any(|f| f.contains("gruond") && f.contains("ground, hero, pickups")),
        "{failures:?}"
    );
}
