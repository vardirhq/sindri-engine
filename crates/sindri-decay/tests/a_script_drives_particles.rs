//! Effects.play, Effects.stop and Effects.emit on a `sindri.particles` emitter.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptReport, ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_scene::{Effects2d, ParticlesComponent, SceneExtractor};

fn registry() -> ComponentSchemaRegistry {
    let mut registry = SceneExtractor::new()
        .expect("the builtin components register")
        .components()
        .clone();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
}

fn emitter(world: &mut World, playing: bool) -> EntityId {
    world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            position: [2.0, 3.0, 0.0],
            ..Transform3D::default()
        }),
        components: [
            (
                ScriptComponent::TYPE_NAME.to_owned(),
                json!({ "source": "dust.decay", "script": "Dust" }),
            ),
            (
                ParticlesComponent::TYPE_NAME.to_owned(),
                json!({ "texture": "dust.png", "playing": playing, "speed": 0.0 }),
            ),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    })
}

fn run(world: &mut World, source: &str, effects: &mut Effects2d) -> ScriptReport {
    let mut sources = ScriptSources::new();
    sources.insert("dust.decay", source);
    let input = InputState::default();
    Scripts::new().advance(
        world,
        &registry(),
        ScriptFrame::new(&sources, &input, 1.0 / 60.0).with_effects(effects),
    )
}

fn playing(world: &World, entity: EntityId) -> bool {
    world.get(entity).unwrap().components[ParticlesComponent::TYPE_NAME]["playing"]
        .as_bool()
        .unwrap()
}

#[test]
fn a_script_plays_and_stops_an_emitter() {
    let mut world = World::default();
    let dust = emitter(&mut world, false);
    let mut effects = Effects2d::default();
    let play = "script Dust { fn start() { Effects.play(this.entity); } }";
    let report = run(&mut world, play, &mut effects);
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    assert!(playing(&world, dust));
    let stop = "script Dust { fn start() { Effects.stop(this.entity); } }";
    let report = run(&mut world, stop, &mut effects);
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    assert!(!playing(&world, dust));
}

#[test]
fn a_script_throws_a_handful_where_the_emitter_is() {
    let mut world = World::default();
    let dust = emitter(&mut world, false);
    let mut effects = Effects2d::default();
    let emit = "script Dust { fn start() { Effects.emit(this.entity, 6.0); } }";
    let report = run(&mut world, emit, &mut effects);
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    assert_eq!(effects.emitter_live(dust), 6);
    let at = effects.flecks()[0].position;
    assert!(
        (at[0] - 2.0).abs() < 1.0e-5 && (at[1] - 3.0).abs() < 1.0e-5,
        "{at:?}"
    );
}

#[test]
fn an_entity_without_particles_is_named() {
    let mut world = World::default();
    let entity = emitter(&mut world, false);
    world
        .get_mut(entity)
        .unwrap()
        .components
        .remove(ParticlesComponent::TYPE_NAME);
    let mut effects = Effects2d::default();
    let play = "script Dust { fn start() { Effects.play(this.entity); } }";
    let report = run(&mut world, play, &mut effects);
    assert!(
        format!("{:?}", report.failures).contains("no particles"),
        "{:?}",
        report.failures
    );
}
