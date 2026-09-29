//! A struct's field defaults, declared in one file, used by scripts in
//! another: a field left out holds its default, even one worked out from a
//! third file's shared constant.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFailure, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;

const TUNING: &str = "shared const BASE: f32 = 2.0;";

const CARDS: &str = "struct Card { name: String, weight: f32 = BASE * 3.0 }";

const USES: &str = r#"script Uses {
    fn update(dt: f32) { this.transform.position.x = Card(name: "a").weight; }
}"#;

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
    sources.insert("cards.decay", CARDS);
    sources.insert("uses.decay", USES);
    sources
}

#[test]
fn a_field_left_out_holds_a_default_another_file_declares() {
    let mut world = World::default();
    let uses = scripted(&mut world, "uses.decay", "Uses");
    let mut scripts = Scripts::new();
    let failures = frame(&mut scripts, &mut world, &sources());
    assert!(failures.is_empty(), "{failures:?}");
    assert!((x(&world, uses) - 6.0).abs() < 1e-6, "{}", x(&world, uses));
}

#[test]
fn changing_the_constant_a_default_uses_changes_what_is_built() {
    let mut world = World::default();
    let uses = scripted(&mut world, "uses.decay", "Uses");
    let mut scripts = Scripts::new();
    let mut sources = sources();
    assert!(frame(&mut scripts, &mut world, &sources).is_empty());
    sources.insert("tuning.decay", "shared const BASE: f32 = 5.0;");
    let failures = frame(&mut scripts, &mut world, &sources);
    assert!(failures.is_empty(), "{failures:?}");
    assert!((x(&world, uses) - 15.0).abs() < 1e-6, "{}", x(&world, uses));
}
