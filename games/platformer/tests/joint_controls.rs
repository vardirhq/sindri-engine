//! Decay changes tether length and releases/reconnects the lantern.
use platformer::Run;
use sindri_platform::Key;

fn step(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        let notes = run.step(1.0 / 60.0);
        assert!(notes.is_empty(), "{notes:?}");
    }
}

fn press(run: &mut Run, key: Key) {
    run.key(key, true);
    step(run, 1);
    run.key(key, false);
    step(run, 1);
}

fn length(run: &Run) -> f32 {
    let [x, y] = run.position(run.entity("wind-lantern").unwrap());
    let [hx, hy] = run.position(run.entity("lantern-anchor").unwrap());
    (x - hx).hypot(y - hy)
}

#[test]
fn lantern_controls_retune_suspend_and_reconnect_the_authored_tether() {
    let mut run = Run::open().unwrap();
    step(&mut run, 180);
    assert!(length(&run) > 1.9);
    press(&mut run, Key::T);
    step(&mut run, 180);
    assert!(length(&run) < 1.3);
    press(&mut run, Key::L);
    step(&mut run, 30);
    assert_eq!(run.physics.world().joint_count(), 3);
    assert!(length(&run) > 3.0);
    let cord = run.entity("lantern-cord").unwrap();
    assert!(run.world.get(cord).unwrap().transform_3d.unwrap().scale[0].abs() < f32::EPSILON);
    press(&mut run, Key::L);
    step(&mut run, 180);
    assert_eq!(run.physics.world().joint_count(), 4);
    assert!(length(&run) < 1.3);
    press(&mut run, Key::T);
    step(&mut run, 180);
    assert!(length(&run) > 1.9 && length(&run) < 2.04);
}
