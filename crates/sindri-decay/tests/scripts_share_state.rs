//! A project's declared state: one file declares `Game.score`, others read
//! and write it by name, and it lives on the board an older script reads.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFailure, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;

const STATE: &str = "state Game {
    var score: f32 = 5.0;
    var won: bool = false;
    let target: f32 = 8.0;
}
state Tuning { let step: f32 = 1.0; }";

/// Adds to the score each frame, and says when it has won.
const SCORER: &str = "script Scorer {
    fn update(dt: f32) {
        Game.score += Tuning.step;
        if Game.score >= Game.target { Game.won = true; }
    }
}";

/// Shows the score, both ways it can be read.
const HUD: &str = "script Hud {
    fn update(dt: f32) {
        this.transform.position.x = Game.score;
        this.transform.position.y = Game.get(\"score\", 0.0);
        if Game.won { this.transform.position.z = 1.0; }
    }
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

fn sources() -> ScriptSources {
    let mut sources = ScriptSources::new();
    sources.insert("state.decay", STATE);
    sources.insert("scorer.decay", SCORER);
    sources.insert("hud.decay", HUD);
    sources
}

fn run(world: &mut World, sources: &ScriptSources, frames: usize) -> (Scripts, Vec<ScriptFailure>) {
    let mut scripts = Scripts::new();
    let mut failures = Vec::new();
    for _ in 0..frames {
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
    (scripts, failures)
}

fn position(world: &World, entity: EntityId) -> [f32; 3] {
    world
        .get(entity)
        .and_then(|data| data.transform_3d)
        .expect("a transform")
        .position
}

#[test]
fn state_starts_as_declared_and_is_shared_with_the_board() {
    let mut world = World::default();
    scripted(&mut world, "scorer.decay", "Scorer");
    let hud = scripted(&mut world, "hud.decay", "Hud");
    let (scripts, failures) = run(&mut world, &sources(), 3);
    assert!(failures.is_empty(), "{failures:?}");
    // 5, then +1 a frame for three frames, the scorer running first.
    let shown = position(&world, hud);
    for (got, want) in shown.iter().zip([8.0, 8.0, 1.0]) {
        assert!((got - want).abs() < 1e-6, "{shown:?}");
    }
    assert!((scripts.blackboard().get("score", 0.0) - 8.0).abs() < 1e-9);
    assert!((scripts.blackboard().get("won", 0.0) - 1.0).abs() < 1e-9);
    // A state of another name keeps its fields under its own name.
    assert!(!scripts.blackboard().has("step"));
}

#[test]
fn an_older_script_writing_the_board_is_seen_through_the_state() {
    let mut world = World::default();
    let hud = scripted(&mut world, "hud.decay", "Hud");
    scripted(&mut world, "old.decay", "Old");
    let mut sources = sources();
    sources.insert(
        "old.decay",
        "script Old { fn update(dt: f32) { Game.set(\"score\", 40.0); } }",
    );
    let (_, failures) = run(&mut world, &sources, 2);
    assert!(failures.is_empty(), "{failures:?}");
    assert!((position(&world, hud)[0] - 40.0).abs() < 1e-6);
}

#[test]
fn a_mistake_about_state_in_another_file_does_not_compile() {
    let mut world = World::default();
    scripted(&mut world, "scorer.decay", "Scorer");
    let mut sources = sources();
    sources.insert(
        "scorer.decay",
        "script Scorer { fn update(dt: f32) { Game.scroe += 1.0; Game.target = 2.0; } }",
    );
    let (_, failures) = run(&mut world, &sources, 1);
    let text = format!("{failures:?}");
    assert!(text.contains("scroe"), "{text}");
    assert!(text.contains("is a `let`"), "{text}");
}

#[test]
fn a_state_field_declared_in_two_files_does_not_compile() {
    let mut world = World::default();
    scripted(&mut world, "scorer.decay", "Scorer");
    let mut sources = sources();
    sources.insert("more.decay", "state Game { var score: f32 = 0.0; }");
    let (_, failures) = run(&mut world, &sources, 1);
    assert!(
        format!("{failures:?}").contains("declared more than once"),
        "{failures:?}"
    );
}
