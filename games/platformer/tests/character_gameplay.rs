//! Controller adoption preserves the hero's Decay movement policies.
use platformer::Run;
use sindri_platform::Key;

const STEP: f32 = 1.0 / 60.0;

fn advance(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        assert!(run.step(STEP).is_empty());
    }
}

fn jump_peak(held_frames: usize) -> f32 {
    let mut run = Run::open().unwrap();
    let hero = run.entity("hero").unwrap();
    advance(&mut run, 90);
    run.key(Key::Space, true);
    let mut peak = run.position(hero)[1];
    for frame in 0..120 {
        if frame == held_frames {
            run.key(Key::Space, false);
        }
        advance(&mut run, 1);
        peak = peak.max(run.position(hero)[1]);
    }
    assert!(run.physics().character_motion(hero).unwrap().grounded);
    peak
}

#[test]
fn releasing_jump_early_still_makes_a_smaller_hop() {
    let hop = jump_peak(3);
    let jump = jump_peak(40);
    assert!(hop > 3.8, "the tap leaves the floor: {hop}");
    assert!(jump > hop + 1.0, "tap {hop}, held {jump}");
}

#[test]
fn running_accelerates_and_releasing_brakes() {
    let mut run = Run::open().unwrap();
    let hero = run.entity("hero").unwrap();
    advance(&mut run, 90);
    let start = run.position(hero)[0];
    run.key(Key::ArrowRight, true);
    // The first script pass queues movement for the following fixed step.
    advance(&mut run, 2);
    let first = run.position(hero)[0] - start;
    advance(&mut run, 6);
    let before = run.position(hero)[0];
    advance(&mut run, 1);
    let cruising = run.position(hero)[0] - before;
    assert!(first > 0.0 && first < cruising * 0.5);
    run.key(Key::ArrowRight, false);
    advance(&mut run, 10);
    let stopped = run.position(hero)[0];
    advance(&mut run, 10);
    assert!((run.position(hero)[0] - stopped).abs() < 0.001);
}

#[test]
fn falling_respawns_and_clears_script_velocity() {
    let mut run = Run::open().unwrap();
    let hero = run.entity("hero").unwrap();
    advance(&mut run, 90);
    run.key(Key::ArrowRight, true);
    advance(&mut run, 10);
    run.key(Key::ArrowRight, false);
    run.world
        .get_mut(hero)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .position = [15.0, -4.0, 0.5];
    advance(&mut run, 90);
    assert!((run.board("falls") - 1.0).abs() < f32::EPSILON);
    let [x, y] = run.position(hero);
    assert!((x - 2.5).abs() < 0.001 && (y - 3.5).abs() < 0.08);
    assert!(run.physics().character_motion(hero).unwrap().grounded);
}
