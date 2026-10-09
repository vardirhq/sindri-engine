//! A run restored to a checkpoint and given the same input steps exactly as
//! it did the first time: what scrubbing a recorded run relies on.

use std::path::Path;

use sindri_core::World;
use sindri_platform::{InputEvent, InputState, Key, MouseButton};
use sindri_runtime::{ProjectRun, STEP};

fn orbital() -> ProjectRun {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/orbital-baked");
    ProjectRun::open(&root, [1280.0, 720.0]).expect("Orbital opens")
}

/// The keys the scripted run holds in turn.
const KEYS: [Key; 4] = [Key::W, Key::D, Key::S, Key::A];

/// Clicks start, then flies about and fires, so the director spawns waves,
/// the random stream moves, and physics and screen UI all carry state.
fn input_for(step: usize, run: &mut ProjectRun) {
    if step == 20 {
        let start = run
            .world
            .entities()
            .find(|(_, data)| data.name.as_deref() == Some("TitleStart"))
            .map(|(entity, _)| entity)
            .expect("the title has a start button");
        let [x, y] = run.on_screen(start).expect("laid out");
        run.input.apply(InputEvent::PointerMoved { x, y });
    }
    if step == 24 {
        run.input
            .apply(InputEvent::ButtonPressed(MouseButton::Left));
    }
    if step == 26 {
        run.input
            .apply(InputEvent::ButtonReleased(MouseButton::Left));
    }
    if step >= 60 && step.is_multiple_of(45) {
        let key = KEYS[(step / 45) % KEYS.len()];
        run.key(key, true);
    }
    if step >= 60 && step % 45 == 30 {
        let key = KEYS[(step / 45) % KEYS.len()];
        run.key(key, false);
    }
    if step >= 90 && step.is_multiple_of(7) {
        run.key(Key::Space, step.is_multiple_of(14));
    }
}

fn shown(world: &World) -> String {
    let mut lines: Vec<String> = world
        .entities()
        .map(|(entity, data)| {
            format!(
                "{entity:?} {} {:?} {}",
                world.is_active(entity),
                data.transform_3d,
                serde_json::to_string(&data.components).expect("serializes")
            )
        })
        .collect();
    lines.sort();
    lines.join("\n")
}

#[test]
fn orbital_restored_to_a_checkpoint_replays_exactly() {
    let mut run = orbital();
    for step in 0..300 {
        input_for(step, &mut run);
        run.step(STEP).expect("steps");
    }
    let world = run.world.clone();
    let session = run.session.checkpoint();
    let mut inputs: Vec<InputState> = Vec::new();
    let mut recorded: Vec<String> = Vec::new();
    for step in 300..600 {
        input_for(step, &mut run);
        inputs.push(run.input.clone());
        run.step(STEP).expect("steps");
        recorded.push(shown(&run.world));
    }
    assert!(
        run.board("run_state") > 0.0,
        "the run got somewhere worth replaying"
    );

    run.world = world;
    run.session.restore(&session);
    for (index, input) in inputs.into_iter().enumerate() {
        run.input = input;
        run.step(STEP).expect("replays");
        assert_eq!(
            shown(&run.world),
            recorded[index],
            "step {} replayed differently",
            300 + index
        );
    }
}
