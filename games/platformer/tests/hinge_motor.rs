//! Decay reverses the windmill motor; the scene keeps its axle fixed.

use platformer::Run;

#[test]
fn the_authored_windmill_turns_and_reverses_through_typed_decay() {
    let mut run = Run::open().unwrap();
    let rotor = run
        .entity("windmill-anchor/mechanism/windmill-rotor")
        .unwrap();
    let anchor = run.entity("windmill-anchor/mechanism").unwrap();
    let mut forwards = false;
    let mut backwards = false;
    for _ in 0..360 {
        let notes = run.step(1.0 / 60.0);
        assert!(notes.is_empty(), "{notes:?}");
        let speed = run.physics.world().angular_velocity(rotor).unwrap();
        forwards |= speed > 1.0;
        backwards |= speed < -1.0;
        let [x, y] = run.position(rotor);
        let [ax, ay] = run.position(anchor);
        assert!((x - ax).hypot(y - ay) < 0.02);
        assert_eq!(run.physics.world().joint_count(), 4);
    }
    assert!(
        forwards && backwards,
        "the scripted motor must reverse the rotor"
    );
    let hinge = run
        .entity("windmill-anchor/mechanism/windmill-hinge")
        .unwrap();
    run.world.despawn_recursive(hinge).unwrap();
    assert!(run.step(1.0 / 60.0).is_empty());
    assert_eq!(run.physics.world().joint_count(), 3);
}
