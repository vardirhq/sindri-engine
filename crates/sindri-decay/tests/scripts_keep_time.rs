//! Timers in play: a timer a script's field holds runs down by each frame
//! before its update, whatever the update then does.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFailure, ScriptFrame, ScriptSources, Scripts};
use sindri_platform::InputState;

const FRAME: f32 = 0.25;

/// Counts the frames until its fuse runs out, and returns early every frame
/// before that, which a hand-written countdown below the return would miss.
const MINE: &str = "script Mine {
    var fuse = Timer(1.0);
    var frames: f32 = 0.0;
    fn update(dt: f32) {
        this.transform.position.y = fuse.left;
        this.transform.position.z = fuse.progress;
        if !fuse.done { frames += 1.0; return; }
        this.transform.position.x = frames;
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

fn frames(world: &mut World, sources: &ScriptSources, count: usize) -> Vec<ScriptFailure> {
    let mut scripts = Scripts::new();
    let mut failures = Vec::new();
    for _ in 0..count {
        failures.extend(
            scripts
                .advance(
                    world,
                    &registry(),
                    ScriptFrame::new(sources, &InputState::default(), FRAME),
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

fn near(got: f32, want: f32) -> bool {
    (got - want).abs() < 1e-5
}

#[test]
fn a_timer_starts_full_and_runs_down_by_each_frame_after() {
    let mut world = World::default();
    let mine = scripted(&mut world, "mine.decay", "Mine");
    let mut sources = ScriptSources::new();
    sources.insert("mine.decay", MINE);

    let failures = frames(&mut world, &sources, 2);
    assert!(failures.is_empty(), "{failures:?}");
    // The first frame starts it full; the second runs it down by one frame.
    let [_, left, progress] = position(&world, mine);
    assert!(
        near(left, 0.75) && near(progress, 0.25),
        "{left} {progress}"
    );
}

#[test]
fn an_early_return_does_not_stop_the_clock() {
    let mut world = World::default();
    let mine = scripted(&mut world, "mine.decay", "Mine");
    let mut sources = ScriptSources::new();
    sources.insert("mine.decay", MINE);

    let failures = frames(&mut world, &sources, 8);
    assert!(failures.is_empty(), "{failures:?}");
    // Full on frame one, then 0.75, 0.5, 0.25, and done on frame five: four
    // frames counted before it, however each update ended.
    let [counted, left, progress] = position(&world, mine);
    assert!(near(counted, 4.0), "{counted}");
    assert!(near(left, 0.0) && near(progress, 1.0), "{left} {progress}");
}

#[test]
fn another_script_reads_a_timer_through_its_type() {
    let mut world = World::default();
    scripted(&mut world, "mine.decay", "Mine");
    let watcher = scripted(&mut world, "watch.decay", "Watch");
    let mut sources = ScriptSources::new();
    sources.insert("mine.decay", MINE);
    sources.insert(
        "watch.decay",
        "script Watch {
            fn update(dt: f32) {
                let mine = Mine.on(World.find(\"Mine\"));
                if mine != null { this.transform.position.x = mine.fuse.duration; }
            }
        }",
    );
    let failures = frames(&mut world, &sources, 2);
    assert!(failures.is_empty(), "{failures:?}");
    assert!(near(position(&world, watcher)[0], 1.0));
}
