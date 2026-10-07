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

#[test]
fn cutting_the_lantern_cord_removes_only_its_authored_constraint() {
    let mut run = Run::open().unwrap();
    step(&mut run, 120);
    let tether = run.entity("lantern-tether").unwrap();
    let body = run.entity("wind-lantern").unwrap();
    let [_, before] = run.position(body);
    press(&mut run, Key::Z);
    assert!(run.world.contains(tether));
    assert!(
        !run.world
            .get(tether)
            .unwrap()
            .components
            .contains_key("sindri.physics2d.distance_joint")
    );
    assert_eq!(run.physics.world().joint_count(), 3);
    step(&mut run, 20);
    let [_, after] = run.position(body);
    assert!(after < before - 0.2);
    let cord = run.entity("lantern-cord").unwrap();
    assert!(run.world.get(cord).unwrap().transform_3d.unwrap().scale[0].abs() < f32::EPSILON);
    for key in [Key::L, Key::T, Key::R, Key::Z] {
        press(&mut run, key);
    }
    assert_eq!(run.physics.world().joint_count(), 3);
}

#[test]
fn repairing_the_cut_cord_creates_a_new_owned_joint_repeatedly() {
    let mut run = Run::open().unwrap();
    step(&mut run, 120);
    let owner = run.entity("lantern-tether").unwrap();
    let body = run.entity("wind-lantern").unwrap();
    press(&mut run, Key::T);
    press(&mut run, Key::R);
    for _ in 0..2 {
        press(&mut run, Key::Z);
        step(&mut run, 20);
        assert_eq!(run.physics.world().joint_count(), 3);
        press(&mut run, Key::C);
        assert!(run.world.contains(body));
        assert_eq!(run.physics.world().joint_count(), 4);
        let joint = &run.world.get(owner).unwrap().components["sindri.physics2d.distance_joint"];
        assert_eq!(joint["first"], "lantern-alternate-anchor");
        assert_eq!(joint["max_distance"], 1.25);
        step(&mut run, 180);
        let [x, y] = run.position(body);
        assert!((x - 11.0).hypot(y - 8.0) < 1.3);
        let cord = run.entity("lantern-cord").unwrap();
        assert!(run.world.get(cord).unwrap().transform_3d.unwrap().scale[0] > 1.0);
        press(&mut run, Key::C);
        assert_eq!(run.physics.world().joint_count(), 4);
    }
    press(&mut run, Key::L);
    assert_eq!(run.physics.world().joint_count(), 3);
    press(&mut run, Key::L);
    assert_eq!(run.physics.world().joint_count(), 4);
}
