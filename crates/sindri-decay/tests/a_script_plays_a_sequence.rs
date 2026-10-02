//! Sequence: a script starting authored choreography and waiting on its cues.
//!
//! Advanced by hand after the scripts, the order every host uses: a script
//! names a sequence, the advance in the same step starts it, and a cue
//! reached by that advance is answered by `Sequence.cued` on the next step.

use serde_json::json;
use sindri_core::{ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, World};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptReport, ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_scene::{SequenceComponent, Sequences};

const STEP: f32 = 0.25;

fn registry() -> ComponentSchemaRegistry {
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
        .register::<SequenceComponent>("Sequence")
        .expect("sindri.sequence registers");
    registry
}

/// A director that runs `script`, with a one-second slide of its child.
fn stage(world: &mut World) -> (EntityId, EntityId) {
    let director = world.spawn(EntityData {
        name: Some("Director".to_owned()),
        components: [
            (
                ScriptComponent::TYPE_NAME.to_owned(),
                json!({ "source": "director.decay", "script": "Director" }),
            ),
            (
                SequenceComponent::TYPE_NAME.to_owned(),
                json!({
                    "playing": null,
                    "sequences": {
                        "intro": {
                            "duration": 1.0,
                            "tracks": [{
                                "target": "Title",
                                "property": "position.y",
                                "keys": [
                                    { "time": 0.0, "value": 5.0 },
                                    { "time": 0.5, "value": 0.0, "ease": "ease-out" }
                                ]
                            }],
                            "cues": [{ "time": 0.5, "name": "landed" }]
                        }
                    }
                }),
            ),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let title = world.spawn(EntityData {
        name: Some("Title".to_owned()),
        ..EntityData::default()
    });
    world.set_parent(title, Some(director)).expect("parent");
    (director, title)
}

const SCRIPT: &str = r#"
script Director {
    fn update(dt: f32) {
        Sequence.play(this.entity, "intro");
        if Sequence.cued(this.entity, "landed") {
            print("landed at " + Sequence.time(this.entity));
        }
        if Sequence.is_finished(this.entity) {
            print("done: " + Sequence.playing(this.entity));
        }
    }
}
"#;

fn step(
    scripts: &mut Scripts,
    world: &mut World,
    sources: &ScriptSources,
    sequences: &mut Sequences,
) -> (ScriptReport, Vec<String>) {
    let input = InputState::default();
    let components = registry();
    let report = scripts.advance(
        world,
        &components,
        ScriptFrame::new(sources, &input, STEP).with_sequences(sequences),
    );
    let fired = sequences
        .advance(world, &components, STEP)
        .expect("the sequences advance");
    let cues = fired.cues.into_iter().map(|cue| cue.name).collect();
    (report, cues)
}

#[test]
fn a_script_starts_a_sequence_and_hears_its_cue() {
    let mut world = World::default();
    let (director, title) = stage(&mut world);
    let mut sources = ScriptSources::new();
    sources.insert("director.decay", SCRIPT);
    let mut scripts = Scripts::new();
    let mut sequences = Sequences::new();

    let (report, cues) = step(&mut scripts, &mut world, &sources, &mut sequences);
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    assert!(cues.is_empty());
    let y = world
        .get(title)
        .and_then(|data| data.transform_3d)
        .expect("moved")
        .position[1];
    assert!(y > 0.0 && y < 5.0, "a quarter of a second in: {y}");

    let (_, cues) = step(&mut scripts, &mut world, &sources, &mut sequences);
    assert_eq!(cues, ["landed"]);
    let (report, _) = step(&mut scripts, &mut world, &sources, &mut sequences);
    let printed: Vec<&str> = report.printed.iter().map(|m| m.message.as_str()).collect();
    assert_eq!(printed, ["landed at 0.5"], "heard on the step after");

    let (_, _) = step(&mut scripts, &mut world, &sources, &mut sequences);
    let (report, _) = step(&mut scripts, &mut world, &sources, &mut sequences);
    let printed: Vec<&str> = report.printed.iter().map(|m| m.message.as_str()).collect();
    assert_eq!(printed, ["done: intro"]);
    assert!(sequences.is_finished(director));
}
