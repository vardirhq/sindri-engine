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

#[test]
fn lantern_retargets_its_hook_without_rebuilding_the_body() {
    let mut run = Run::open().unwrap();
    step(&mut run, 180);
    let body = run.entity("wind-lantern").unwrap();
    let tether = run.entity("lantern-tether").unwrap();
    press(&mut run, Key::R);
    assert_eq!(
        run.world.get(tether).unwrap().components["sindri.physics2d.distance_joint"]["first"],
        "lantern-alternate-anchor"
    );
    assert_eq!(run.physics.world().joint_count(), 4);
    step(&mut run, 180);
    let [x, y] = run.position(body);
    assert!((x - 11.0).hypot(y - 8.0) < 2.04);
    press(&mut run, Key::L);
    press(&mut run, Key::R);
    assert_eq!(run.physics.world().joint_count(), 3);
    assert_eq!(
        run.world.get(tether).unwrap().components["sindri.physics2d.distance_joint"]["first"],
        "lantern-anchor"
    );
    press(&mut run, Key::L);
    step(&mut run, 180);
    assert_eq!(run.physics.world().joint_count(), 4);
    assert!(length(&run) < 2.04);
}
