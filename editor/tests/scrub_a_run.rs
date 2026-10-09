//! A recorded Orbital run scrubbed back to an earlier wave and resumed plays
//! the same range again exactly.
//!
//! The editor's own path, headless: `Recording` keeps the steps and copies as
//! Play does, and going back restores the copy at or before the step and
//! replays the recorded input, presenting after each step the recording says
//! was drawn after. Here every step is presented, as `ProjectRun` presents.

use std::path::Path;

use sindri_core::World;
use sindri_editor::recording::{RecordedStep, Recording};
use sindri_platform::{InputEvent, Key, MouseButton};
use sindri_runtime::{ProjectRun, STEP};

const SIZE: [f32; 2] = [1280.0, 720.0];
const VIEW: weave::Viewport = weave::Viewport {
    width: SIZE[0],
    height: SIZE[1],
};

/// The keys the scripted run holds in turn.
const KEYS: [Key; 4] = [Key::W, Key::D, Key::S, Key::A];

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
        run.key(KEYS[(step / 45) % KEYS.len()], true);
    }
    if step >= 60 && step % 45 == 30 {
        run.key(KEYS[(step / 45) % KEYS.len()], false);
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

/// One step as Play takes it: recorded, stepped, recorded as drawn.
fn play(run: &mut ProjectRun, recording: &mut Recording) {
    recording.before_step(RecordedStep {
        input: run.input.clone(),
        viewport: (SIZE[0], SIZE[1]),
        delta: STEP,
        drawn: None,
    });
    run.step(STEP).expect("steps");
    recording.after_step(&run.world, &run.session);
    recording.drew(VIEW);
}

#[test]
fn orbital_scrubbed_back_a_wave_and_resumed_plays_the_same_again() {
    const LENGTH: usize = 1200;
    const BACK_TO: u64 = 500;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../games/orbital-baked");
    let mut run = ProjectRun::open(&root, SIZE).expect("Orbital opens");
    let mut recording = Recording::start(&run.world, &run.session);
    let mut recorded = Vec::with_capacity(LENGTH);
    let mut elapsed = Vec::with_capacity(LENGTH);
    for step in 0..LENGTH {
        input_for(step, &mut run);
        play(&mut run, &mut recording);
        recorded.push(shown(&run.world));
        elapsed.push(run.board("elapsed"));
    }
    let wave = |seconds: f32| (seconds / 6.0).floor();
    let back = usize::try_from(BACK_TO).unwrap() - 1;
    assert!(
        wave(elapsed[back]) < wave(elapsed[LENGTH - 1]),
        "back to an earlier wave: {} against {}",
        elapsed[back],
        elapsed[LENGTH - 1]
    );

    // Scrub back, as the Timeline does.
    let replay = recording.seek(BACK_TO).expect("recorded");
    assert!(replay.edits.is_empty(), "nothing was edited in this run");
    run.world = replay.mark.world.clone();
    run.session.restore(&replay.mark.session);
    for step in replay.steps {
        run.input = step.input;
        run.step(step.delta).expect("replays");
    }
    recording.scrubbed_to(BACK_TO);
    assert_eq!(
        shown(&run.world),
        recorded[back],
        "the scrubbed step is the recorded one"
    );

    // Resume with the same input: the range plays again exactly.
    let resumed = usize::try_from(BACK_TO).unwrap();
    for (step, expected) in recorded.iter().enumerate().skip(resumed) {
        input_for(step, &mut run);
        play(&mut run, &mut recording);
        assert_eq!(&shown(&run.world), expected, "step {step} differs");
    }
    assert_eq!(
        recording.range().1,
        LENGTH as u64,
        "the future was let go and played again"
    );
}
