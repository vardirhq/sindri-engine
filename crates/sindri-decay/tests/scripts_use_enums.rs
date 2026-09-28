//! Enums across a project: declared in one file, authored by a variant's name
//! in a scene, matched in a script, and kept on the board as a state.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFailure, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;

const KINDS: &str = "enum Kind { Grow, Boost, Wind }
enum Phase { Lobby, Countdown, Play }
state Game { var phase: Phase = Phase.Lobby; }";

/// Shows its kind as a number, and moves the game on a phase each frame.
const POWER: &str = "script Power {
    @export var kind = Kind.Grow;
    fn update(dt: f32) {
        match this.kind {
            Kind.Grow => { this.transform.position.x = 1.0; }
            Kind.Boost => { this.transform.position.x = 2.0; }
            Kind.Wind => { this.transform.position.x = 3.0; }
        }
        match Game.phase {
            Phase.Lobby => { Game.phase = Phase.Countdown; }
            Phase.Countdown => { Game.phase = Phase.Play; }
            Phase.Play => { this.transform.position.y = 1.0; }
        }
    }
}";

fn registry() -> ComponentSchemaRegistry {
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
}

fn power(world: &mut World, kind: Option<&str>) -> EntityId {
    let mut payload = json!({ "source": "power.decay", "script": "Power" });
    if let Some(kind) = kind {
        payload["properties"] = json!({ "kind": kind });
    }
    world.spawn(EntityData {
        name: Some("Power".to_owned()),
        transform_3d: Some(Transform3D::default()),
        components: [(ScriptComponent::TYPE_NAME.to_owned(), payload)]
            .into_iter()
            .collect(),
        ..EntityData::default()
    })
}

fn sources() -> ScriptSources {
    let mut sources = ScriptSources::new();
    sources.insert("kinds.decay", KINDS);
    sources.insert("power.decay", POWER);
    sources
}

fn frames(world: &mut World, scripts: &mut Scripts, count: usize) -> Vec<ScriptFailure> {
    let sources = sources();
    let mut failures = Vec::new();
    for _ in 0..count {
        failures.extend(
            scripts
                .advance(
                    world,
                    &registry(),
                    ScriptFrame::new(&sources, &InputState::default(), 1.0 / 60.0),
                )
                .failures,
        );
    }
    failures
}

fn position(world: &World, entity: EntityId) -> [f32; 3] {
    world
        .get(entity)
        .and_then(|data| data.transform_3d)
        .expect("a transform")
        .position
}

#[test]
fn a_scene_authors_an_enum_by_its_variant_s_name() {
    let mut world = World::default();
    let plain = power(&mut world, None);
    let boost = power(&mut world, Some("Boost"));
    let wind = power(&mut world, Some("Kind.Wind"));
    let mut scripts = Scripts::new();
    let failures = frames(&mut world, &mut scripts, 1);
    assert!(failures.is_empty(), "{failures:?}");
    assert!((position(&world, plain)[0] - 1.0).abs() < 1e-6);
    assert!((position(&world, boost)[0] - 2.0).abs() < 1e-6);
    assert!((position(&world, wind)[0] - 3.0).abs() < 1e-6);
}

#[test]
fn a_name_that_is_no_variant_is_refused_with_the_ones_it_could_be() {
    let mut world = World::default();
    power(&mut world, Some("Shrink"));
    let mut scripts = Scripts::new();
    let failures = frames(&mut world, &mut scripts, 1);
    assert!(
        failures.iter().any(|failure| matches!(
            failure,
            ScriptFailure::Property { property, reason, .. }
                if property == "kind" && reason.contains("Grow, Boost, Wind")
        )),
        "{failures:?}"
    );
}

#[test]
fn the_inspector_is_offered_the_variants() {
    let mut world = World::default();
    power(&mut world, None);
    let mut scripts = Scripts::new();
    assert!(frames(&mut world, &mut scripts, 1).is_empty());
    let exports = scripts.exports("power.decay", "Power").expect("compiled");
    assert_eq!(exports[0].choices, vec!["Grow", "Boost", "Wind"]);
    assert_eq!(exports[0].type_name.as_deref(), Some("Kind"));
}

#[test]
fn an_enum_state_is_kept_on_the_board_as_its_position() {
    let mut world = World::default();
    let entity = power(&mut world, None);
    let mut scripts = Scripts::new();
    // Lobby, then Countdown, then Play; Play shows on the third frame.
    let failures = frames(&mut world, &mut scripts, 3);
    assert!(failures.is_empty(), "{failures:?}");
    assert!((scripts.blackboard().get("phase", -1.0) - 2.0).abs() < 1e-9);
    assert!((position(&world, entity)[1] - 1.0).abs() < 1e-6);
}
