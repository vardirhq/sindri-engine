//! A session stepping a scene: a script moves an entity, what it printed comes
//! back in the step's report, a measuring session times its phases, and the
//! engine's own block set is bound without anyone asking.

use serde_json::json;
use sindri_core::{
    BUILTIN_BLOCKS, ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D,
    World,
};
use sindri_decay::{ScriptComponent, ScriptSources};
use sindri_platform::InputState;
use sindri_runtime::{Session, StepPhase};
use sindri_scene::SceneExtractor;

const WALK: &str = r#"
script Walk {
    var steps: f32 = 0.0;

    fn update(dt: f32) {
        steps += 1.0;
        this.transform.position.x += 1.0;
        if steps == 2.0 {
            print("two steps");
        }
    }
}
"#;

fn registry() -> ComponentSchemaRegistry {
    let mut extractor = SceneExtractor::new().expect("the engine's components register");
    extractor
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    extractor.components().clone()
}

fn walker() -> (World, EntityId) {
    let mut world = World::default();
    let entity = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({ "source": "scripts/walk.decay", "script": "Walk" }),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    (world, entity)
}

fn session() -> Session {
    let mut sources = ScriptSources::new();
    sources.insert("scripts/walk.decay", WALK);
    Session::with_sources(registry(), sources)
}

fn x(world: &World, entity: EntityId) -> f32 {
    world
        .get(entity)
        .and_then(|data| data.transform_3d)
        .expect("the walker kept its transform")
        .position[0]
}

#[test]
fn a_step_runs_the_scripts_and_reports_what_they_said() {
    let (mut world, entity) = walker();
    let mut session = session();
    let input = InputState::default();
    let first = session
        .step(&mut world, &input, (640.0, 360.0), 1.0 / 60.0)
        .expect("the first step runs");
    assert!(
        first.scripts.failures.is_empty(),
        "{:?}",
        first.scripts.failures
    );
    let second = session
        .step(&mut world, &input, (640.0, 360.0), 1.0 / 60.0)
        .expect("the second step runs");
    assert!((x(&world, entity) - 2.0).abs() < 1e-5);
    let said: Vec<&str> = second
        .scripts
        .printed
        .iter()
        .map(|message| message.message.as_str())
        .collect();
    assert_eq!(said, ["two steps"]);
}

#[test]
fn a_measuring_session_times_its_phases_and_scripts() {
    let (mut world, _) = walker();
    let mut session = session();
    let input = InputState::default();
    let quiet = session
        .step(&mut world, &input, (640.0, 360.0), 1.0 / 60.0)
        .expect("a step runs");
    assert!(
        quiet.times.total().is_zero(),
        "nothing is timed unless asked"
    );
    assert!(quiet.scripts.timings.is_empty());
    session.set_measuring(true);
    let timed = session
        .step(&mut world, &input, (640.0, 360.0), 1.0 / 60.0)
        .expect("a step runs");
    assert!(timed.times.phase(StepPhase::Scripts) > std::time::Duration::ZERO);
    assert_eq!(timed.scripts.timings.len(), 1);
}

#[test]
fn the_engines_own_block_set_is_bound() {
    let session = session();
    assert!(session.tile_sets().get(BUILTIN_BLOCKS).is_some());
    let replaced = session.with_tile_sets(sindri_scene::TileSetBindings::new());
    assert!(
        replaced.tile_sets().get(BUILTIN_BLOCKS).is_some(),
        "a host's own sets are bound beside the engine's, not instead of them"
    );
}
