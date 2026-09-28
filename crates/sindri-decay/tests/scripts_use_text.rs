//! Text in a running project: built with `+` from numbers, flags and
//! variants, printed as it reads, and used as a board key.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptReport, ScriptSources, Scripts};
use sindri_platform::InputState;

const SOURCE: &str = r#"enum Phase { Lobby, Play }
state Game { var score: f32 = 0.0; var phase = Phase.Lobby; }
script Label {
    @export let stat: String = "damage";
    var shown = "";
    fn update(dt: f32) {
        Game.score += 1.0;
        this.shown = "Score " + Game.score + " in " + Game.phase;
        print(this.shown.uppercase);
        print(0.1);
        Game.set(this.stat + "_add", this.shown.length);
    }
}"#;

fn registry() -> ComponentSchemaRegistry {
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
}

fn label(world: &mut World) -> EntityId {
    world.spawn(EntityData {
        name: Some("Label".to_owned()),
        transform_3d: Some(Transform3D::default()),
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": "label.decay", "script": "Label", "properties": { "stat": "armor" } }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    })
}

fn frame(world: &mut World, scripts: &mut Scripts) -> ScriptReport {
    let mut sources = ScriptSources::new();
    sources.insert("label.decay", SOURCE);
    scripts.advance(
        world,
        &registry(),
        ScriptFrame::new(&sources, &InputState::default(), 1.0 / 60.0),
    )
}

#[test]
fn a_script_builds_text_and_says_it() {
    let mut world = World::default();
    label(&mut world);
    let mut scripts = Scripts::new();
    frame(&mut world, &mut scripts);
    let report = frame(&mut world, &mut scripts);
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    let printed: Vec<&str> = report
        .printed
        .iter()
        .map(|message| message.message.as_str())
        .collect();
    assert_eq!(printed, ["SCORE 2 IN LOBBY", "0.1"]);
    // "Score 2 in Lobby" is sixteen characters, under a key the scene named.
    assert!((scripts.blackboard().get("armor_add", -1.0) - 16.0).abs() < 1e-9);
}
