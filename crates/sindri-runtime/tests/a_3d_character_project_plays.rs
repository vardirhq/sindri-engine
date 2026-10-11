//! The feature example uses ordinary keyboard input and the shared project host.
use sindri_platform::{InputEvent, Key};
use sindri_runtime::{ProjectRun, STEP};
use std::path::Path;

fn step(run: &mut ProjectRun) -> Vec<String> {
    let report = run.step(STEP).expect("character project steps");
    assert!(report.notes().is_empty(), "{report:?}");
    report
        .scripts
        .printed
        .into_iter()
        .map(|line| line.message)
        .collect()
}

#[test]
fn keyboard_jump_leaves_solved_carry_and_reaches_the_goal() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/character3d");
    let mut run = ProjectRun::open(&path, [960.0, 540.0]).expect("character project opens");
    let actor = run.entity("walker").unwrap();
    let ferry = run.entity("ferry").unwrap();
    let mut said = Vec::new();
    for _ in 0..90 {
        said.extend(step(&mut run));
    }
    assert!(said.iter().any(|line| line == "Character carried"));
    let before = run.world.world_transform(actor).unwrap().position;
    assert!((before[0] - 1.5).abs() < 0.03, "solved carry: {before:?}");
    assert_eq!(
        run.session
            .physics3d()
            .character_motion(actor)
            .unwrap()
            .platform
            .as_ref()
            .unwrap()
            .entity,
        ferry
    );
    run.input.apply(InputEvent::KeyPressed(Key::Space));
    run.input.apply(InputEvent::KeyPressed(Key::D));
    said.extend(step(&mut run));
    run.input.apply(InputEvent::KeyReleased(Key::Space));
    said.extend(step(&mut run));
    assert!(run.world.world_transform(actor).unwrap().position[1] > before[1]);
    // The jump request is applied after the next solve's final support carry.
    said.extend(step(&mut run));
    assert!(
        run.session
            .physics3d()
            .character_motion(actor)
            .unwrap()
            .platform
            .is_none(),
        "the airborne step has no carry support"
    );
    for _ in 0..180 {
        said.extend(step(&mut run));
        if said.iter().any(|line| line == "Character goal reached") {
            break;
        }
    }
    run.input.apply(InputEvent::KeyReleased(Key::D));
    for expected in [
        "Character jumped",
        "Character landed on floor",
        "Character goal reached",
    ] {
        assert!(
            said.iter().any(|line| line == expected),
            "missing {expected}: {said:?}"
        );
    }
    let position = run.world.world_transform(actor).unwrap().position;
    assert!(
        (position[1] - 0.31).abs() < 0.03,
        "landed pose: {position:?}"
    );
}

#[test]
fn an_idle_rider_follows_the_decay_ferrys_reversal_once() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/character3d");
    let mut run = ProjectRun::open(&path, [960.0, 540.0]).unwrap();
    let actor = run.entity("walker").unwrap();
    let ferry = run.entity("ferry").unwrap();
    let mut previous = 0.0;
    let mut reversed = false;
    for _ in 0..240 {
        step(&mut run);
        let position = run.world.world_transform(actor).unwrap().position;
        let platform = run.world.world_transform(ferry).unwrap().position;
        assert!(
            (position[0] - platform[0]).abs() < 0.002,
            "carry applied once"
        );
        assert!(
            (position[1] - 1.31).abs() < 0.02,
            "classified support holds"
        );
        if platform[0] < previous {
            reversed = true;
        }
        previous = platform[0];
    }
    assert!(reversed, "Decay reverses the solved platform");
}
